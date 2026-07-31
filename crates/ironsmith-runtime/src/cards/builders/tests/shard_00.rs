#![allow(unused_imports)]
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
use super::shard_10::*;
use super::shard_11::*;
use super::shard_12::*;
use super::shard_13::*;
use super::shard_14::*;
use super::shard_15::*;
use super::shard_16::*;
use super::shard_17::*;
use super::shard_18::*;
use super::shard_19::*;
use super::shard_20::*;
use super::shard_21::*;
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[test]
pub(super) fn test_creature_with_keywords() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test Creature")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Angel])
        .power_toughness(PowerToughness::fixed(3, 3))
        .flying()
        .vigilance()
        .build();

    assert_eq!(def.name(), "Test Creature");
    assert!(def.is_creature());
    assert_eq!(def.abilities.len(), 2);
}

#[test]
pub(super) fn test_creature_with_mana_ability() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mana Dork")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Green]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Elf, Subtype::Druid])
        .power_toughness(PowerToughness::fixed(1, 1))
        .taps_for(ManaSymbol::Green)
        .build();

    assert!(def.is_creature());
    assert_eq!(def.abilities.len(), 1);
    assert!(def.abilities[0].is_mana_ability());
}

#[test]
pub(super) fn test_spell_with_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test Bolt")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::deal_damage(3, ChooseSpec::AnyTarget)])
        .build();

    assert!(def.is_spell());
    assert!(def.spell_effect.is_some());
    assert_eq!(def.spell_effect.as_ref().unwrap().len(), 1);
}

#[test]
pub(super) fn parse_all_creature_type_gain_and_loss_effects() {
    let amoeboid = CardDefinitionBuilder::new(CardId::from_raw(1), "Amoeboid Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Target creature gains all creature types until end of turn.\n\
             {T}: Target creature loses all creature types until end of turn.",
        )
        .expect("parse all-creature-type activated abilities");
    let amoeboid_debug = format!("{:?}", amoeboid.abilities);
    assert!(
        amoeboid_debug.contains("AddAllSubtypesOfFamily")
            && amoeboid_debug.contains("RemoveAllSubtypesOfFamily"),
        "expected all-subtype gain and loss continuous effects, got {amoeboid_debug}"
    );

    let inversion = CardDefinitionBuilder::new(CardId::from_raw(2), "Nameless Probe")
        .card_types(vec![CardType::Kindred, CardType::Instant])
        .parse_text("Target creature gets +3/-3 and loses all creature types until end of turn.")
        .expect("parse pump plus all-creature-type loss spell");
    let spell_debug = format!("{:?}", inversion.spell_effect);
    assert!(
        spell_debug.contains("ModifyPowerToughness")
            && spell_debug.contains("RemoveAllSubtypesOfFamily"),
        "expected pump and all-subtype loss effects, got {spell_debug}"
    );
}

#[test]
pub(super) fn parse_target_player_controls_pump_uses_captured_controller_filter() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Target Pump Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Creatures target player controls get +1/+1 and gain first strike and haste until end of turn.")
        .expect("parse target-player-controlled pump clause");
    let debug = format!("{:?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();

    assert!(
        debug.contains("ApplyContinuousEffect")
            && debug.contains("controller: Some(Target(Any))")
            && debug.contains("ModifyPowerToughness")
            && debug.contains("AddAbility")
            && debug.contains("FirstStrike")
            && debug.contains("Haste"),
        "target-player-controlled pump should lower to filter-scoped pump plus ability grant, got {debug}"
    );
    assert!(
        rendered.contains("creatures target player controls get +1/+1")
            && rendered.contains("first strike")
            && rendered.contains("haste"),
        "target-player-controlled pump should preserve surface text, got {rendered}"
    );
}

#[test]
pub(super) fn parse_damage_replacement_counter_clause_uses_captured_target() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(2), "Prevention Counter Probe")
        .card_types(vec![CardType::Instant])
        .parse_text("If damage would be dealt to target creature this turn, prevent that damage and put that many +1/+1 counters on it.")
        .expect("parse damage-prevention counter replacement clause");
    let debug = format!("{:?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();

    assert!(
        debug.contains("PreventAllDamageToTargetEffect")
            && debug.contains("PutCountersEffect")
            && debug.contains("PlusOnePlusOne"),
        "damage replacement counter clause should lower to target-scoped prevention with counter follow-up, got {debug}"
    );
    assert!(
        rendered.contains("prevent all damage that would be dealt to target creature this turn")
            && rendered.contains("for each 1 damage prevented this way")
            && rendered.contains("put a +1/+1 counter on that creature"),
        "damage replacement counter clause should preserve prevention/counter surface, got {rendered}"
    );
}

#[test]
pub(super) fn parse_clash_repeat_process_spell_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(3), "Clash Repeat Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You lose 2 life and draw two cards, then clash with an opponent. If you win, repeat this process.",
        )
        .expect("parse repeated clash spell");
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("RepeatProcessEffect")
            && debug.contains("LoseLifeEffect")
            && debug.contains("DrawCardsEffect")
            && debug.contains("ClashEffect"),
        "expected repeated lose/draw/clash process, got {debug}"
    );
}

#[test]
pub(super) fn last_rites_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Last Rites");

    let def = parse_oracle_card_definition("Last Rites");
    let spell_debug = format!("{:#?}", def.spell_effect);
    let rendered = compiled_text_lines(&def).join(" ");

    assert!(
        spell_debug.contains("DiscardEffect")
            && spell_debug.contains("any_number: true")
            && spell_debug.contains("LookAtHandEffect")
            && spell_debug.contains("ChooseObjectsEffect")
            && spell_debug.contains("count_value: Some")
            && spell_debug.contains("Count(")
            && spell_debug.contains("discarded_this_way"),
        "expected Last Rites to discard any number, reveal hand, choose a dynamic count, and discard chosen cards, got {spell_debug}"
    );
    assert!(
        rendered.contains("Discard any number of cards. Target player reveals their hand, then you choose a nonland card from it for each card discarded this way. That player discards those cards"),
        "expected Last Rites oracle surface to be preserved, got {rendered}"
    );
}

#[test]
pub(super) fn aligned_heart_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Aligned Heart");

    let def = parse_oracle_card_definition("Aligned Heart");
    let rendered = compiled_text_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "Whenever you cast your second spell each turn, put a rally counter on this enchantment. Then create a 1/1 white Monk creature token with prowess for each rally counter on it"
        ),
        "Aligned Heart should render the rally-counter token count with token prowess inline, got {rendered}"
    );
    assert!(
        ability_debug.contains("SpellCast")
            && ability_debug.contains("exact_spells_this_turn: Some")
            && ability_debug.contains("rally")
            && ability_debug.contains("CountersOnSource")
            && ability_debug.contains("CreateTokenEffect"),
        "Aligned Heart should lower flurry into a second-spell trigger that puts a named rally counter and creates tokens from that counter count, got {ability_debug}"
    );
}

#[test]
pub(super) fn kodama_of_the_center_tree_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Kodama of the Center Tree");

    let def = parse_oracle_card_definition("Kodama of the Center Tree");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "Kodama of the Center Tree's power and toughness are each equal to the number of Spirits you control."
        ) && rendered.contains(
                "Kodama of the Center Tree has soulshift X, where X is the number of Spirits you control."
        ),
        "expected Kodama compiled text to preserve CDA and dynamic soulshift surfaces, got {rendered}"
    );
    assert!(
        !rendered.contains("Kodama's power") && !rendered.contains("Kodama has soulshift"),
        "a legendary with no captured oracle short name must keep its full self-reference, got {rendered}"
    );
    assert!(
        ability_debug.contains("LessThanOrEqualExpr")
            && ability_debug.contains("Count(ObjectFilter")
            && ability_debug.contains("subtypes: [Spirit]"),
        "expected Kodama soulshift target cap to count Spirits you control, got {ability_debug}"
    );
}

#[test]
pub(super) fn rayne_academy_chancellor_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Rayne, Academy Chancellor");

    let def = parse_oracle_card_definition("Rayne, Academy Chancellor");
    let rendered = compiled_text_lines(&def).join(" ");
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains(
            "Whenever you or a permanent you control becomes the target of a spell or ability an opponent controls, you may draw a card"
        ),
        "expected Rayne's player-or-permanent opponent-controlled targeting trigger to render, got {rendered}"
    );
    assert!(
        rendered.contains("You may draw an additional card if this creature is enchanted"),
        "expected Rayne's enchanted additional-draw clause to render, got {rendered}"
    );
    assert!(
        abilities_debug.contains("SourceIsEnchanted") && abilities_debug.contains("MayEffect"),
        "expected Rayne's additional draw to be a conditional optional effect, got {abilities_debug}"
    );
}

#[test]
pub(super) fn will_kenrith_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Will Kenrith");

    let oracle = oracle_text_by_name()
        .get("Will Kenrith")
        .expect("Will Kenrith oracle text should exist")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Will Kenrith")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Planeswalker])
        .loyalty(4)
        .parse_text(oracle)
        .expect("Will Kenrith oracle text should parse");
    let rendered = compiled_text_lines(&def).join("\n");
    let abilities_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.contains("+2: Until your next turn, up to two target creatures each have base power and toughness 0/3")
            && rendered.contains("up to two target creatures lose all abilities"),
        "expected Will Kenrith +2 compiled text to preserve shared base P/T and lose-abilities clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Target player draws two cards. Until your next turn, instant, sorcery, and planeswalker spells that player casts cost {2} less to cast."
        ),
        "expected Will Kenrith -2 compiled text to preserve target-player cost reduction, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Target player gets an emblem with \"Whenever you cast an instant or sorcery spell, copy it. You may choose new targets for the copy."
        ),
        "expected Will Kenrith -8 compiled text to preserve target-player emblem, got {rendered}"
    );
    assert!(
        rendered.contains("Partner with Rowan Kenrith")
            && rendered.contains("Will Kenrith can be your commander."),
        "expected Will Kenrith commander text to render, got {rendered}"
    );
    assert!(
        abilities_debug.contains("SetPowerToughness")
            && abilities_debug.contains("RemoveAllAbilities")
            && abilities_debug.contains("max: Some")
            && abilities_debug.contains("GrantNextSpellCostReductionEffect")
            && abilities_debug.contains("applies_to_all_matching_this_turn: true")
            && abilities_debug.contains("duration: YourNextTurn")
            && abilities_debug.contains("CreateEmblemEffect"),
        "expected Will Kenrith to model shared targets, timed cost reduction, and target-player emblem, got {abilities_debug}"
    );
}

#[test]
pub(super) fn rayne_academy_chancellor_targeting_trigger_draws_conditionally_at_runtime() {
    struct AcceptMayDecisionMaker;
    impl crate::decision::DecisionMaker for AcceptMayDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &crate::game_state::GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    fn alice_hand_count(game: &crate::game_state::GameState, alice: PlayerId) -> usize {
        game.objects_in_zone(Zone::Hand)
            .into_iter()
            .filter_map(|id| game.object(id))
            .filter(|object| game.controller_of(object) == alice)
            .count()
    }

    fn resolve_rayne_targeting_trigger(
        enchanted: bool,
        target_player: bool,
        source_controller: PlayerId,
    ) -> (usize, usize) {
        let def = parse_oracle_card_definition("Rayne, Academy Chancellor");
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let rayne = game.create_object_from_definition(&def, alice, Zone::Battlefield);

        for idx in 0..2 {
            let draw_card = CardDefinitionBuilder::new(CardId::new(), format!("Rayne Draw {idx}"))
                .card_types(vec![CardType::Creature])
                .build();
            game.create_object_from_definition(&draw_card, alice, Zone::Library);
        }

        if enchanted {
            let aura = CardDefinitionBuilder::new(CardId::new(), "Rayne Test Aura")
                .card_types(vec![CardType::Enchantment])
                .subtypes(vec![Subtype::Aura])
                .enchants(crate::target::ObjectFilter::creature())
                .build();
            let aura_id = game.create_object_from_definition(&aura, alice, Zone::Battlefield);
            game.object_mut(aura_id)
                .expect("Rayne test Aura should exist")
                .attached_to = Some(crate::object::AttachmentTarget::Object(rayne));
            game.object_mut(rayne)
                .expect("Rayne should exist")
                .attachments
                .push(aura_id);
        }

        let spell = CardDefinitionBuilder::new(CardId::new(), "Rayne Targeting Spell")
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_definition(&spell, source_controller, Zone::Stack);
        let target = if target_player {
            crate::game_state::Target::Player(alice)
        } else {
            crate::game_state::Target::Object(rayne)
        };
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::BecomesTargetedEvent::new_target(
                target,
                spell_id,
                source_controller,
                false,
            ),
            crate::provenance::ProvNodeId::default(),
        );

        let triggers = crate::triggers::check_triggers(&game, &event);
        let matching_count = triggers
            .iter()
            .filter(|entry| entry.source == rayne)
            .count();
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        for trigger in triggers.into_iter().filter(|entry| entry.source == rayne) {
            trigger_queue.add(trigger);
        }
        if matching_count > 0 {
            let mut dm = AcceptMayDecisionMaker;
            crate::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut trigger_queue, &mut dm)
                .expect("Rayne trigger should go on the stack");
            crate::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
                .expect("Rayne trigger should resolve");
        }

        (matching_count, alice_hand_count(&game, alice))
    }

    let bob = PlayerId::from_index(1);
    let alice = PlayerId::from_index(0);

    assert_eq!(
        resolve_rayne_targeting_trigger(false, false, bob),
        (1, 1),
        "Rayne should draw one card when unenchanted and targeted by an opponent's source"
    );
    assert_eq!(
        resolve_rayne_targeting_trigger(true, false, bob),
        (1, 2),
        "Rayne should draw the additional optional card while enchanted"
    );
    assert_eq!(
        resolve_rayne_targeting_trigger(false, true, bob),
        (1, 1),
        "Rayne should draw when its controller becomes targeted by an opponent's source"
    );
    assert_eq!(
        resolve_rayne_targeting_trigger(true, false, alice),
        (0, 0),
        "Rayne should not trigger from a source its controller controls"
    );
}

#[test]
pub(super) fn duplicant_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Duplicant");

    let def = parse_oracle_card_definition("Duplicant");
    let rendered = compiled_text_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);
    let static_ids = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let source_static_debug = def
        .abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return None;
            };
            (static_ability.id() == StaticAbilityId::SourceCharacteristicsOfLastExiledCreatureCard)
                .then(|| format!("{static_ability:#?}"))
        })
        .expect("Duplicant should have source-linked static characteristics");

    assert!(
        rendered.contains("When this creature enters, you may exile target nontoken creature"),
        "expected Duplicant's optional nontoken exile trigger to render, got {rendered}"
    );
    assert!(
        rendered.contains("last creature card exiled with it")
            && rendered.contains("It's still a Shapeshifter"),
        "expected Duplicant's exiled-card characteristic static ability to render, got {rendered}"
    );
    assert!(
        ability_debug.contains("MayEffect")
            && ability_debug.contains("nontoken: true")
            && static_ids.contains(&StaticAbilityId::SourceCharacteristicsOfLastExiledCreatureCard),
        "expected Duplicant to model optional nontoken exile plus source-linked static characteristics, got ids {static_ids:?} and abilities {ability_debug}"
    );
    assert!(
        source_static_debug.contains("nontoken: true")
            && source_static_debug.contains("zone: Some(Exile)"),
        "expected Duplicant's source-linked static filter to require nontoken creature cards in exile, got {source_static_debug}"
    );
}

#[test]
pub(super) fn rampaging_aetherhood_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Rampaging Aetherhood");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Rampaging Aetherhood should parse its upkeep trigger strictly"
    );
    assert!(
        ability_debug.contains("EnergyCountersEffect")
            && ability_debug.contains("PayAnyEnergyEffect")
            && ability_debug.contains("min_amount: 1")
            && ability_debug.contains("IfEffect")
            && ability_debug.contains("PutCountersEffect"),
        "expected energy payment and paid-amount counter effects, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "Then you may pay one or more {E}. If you do, put that many +1/+1 counters on this creature"
        ),
        "expected one-or-more energy payment text to be preserved, got {rendered}"
    );
}

#[test]
pub(super) fn aether_refinery_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Aether Refinery");

    let def = parse_oracle_card_definition("Aether Refinery");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        ability_debug.contains("DoubleCountersReplacement")
            && ability_debug.contains("Energy")
            && ability_debug.contains("PayAnyEnergyEffect")
            && ability_debug.contains("min_amount: 1")
            && ability_debug.contains("CreateTokenEffect"),
        "expected player energy replacement and paid-energy token activation, got {ability_debug}"
    );
    assert!(
        rendered.contains("If you would get one or more {E}, you get twice that many {E} instead.")
            && rendered.contains(
                "{T}: You get {E}, then you may pay one or more {E}. If you do, create an X/X black Aetherborn creature token, where X is the amount of {E} paid this way."
            ),
        "expected Aether Refinery compiled text to preserve energy replacement and activation surface, got {rendered}"
    );
}

#[test]
pub(super) fn feast_of_the_victorious_dead_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Feast of the Victorious Dead");

    let def = parse_oracle_card_definition("Feast of the Victorious Dead");
    let rendered = compiled_text_lines(&def).join(" ");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Feast of the Victorious Dead should have an end-step trigger");

    assert!(
        matches!(
            triggered.intervening_if,
            Some(
                crate::effect::Condition::CreatureDiedThisTurn
                    | crate::effect::Condition::CreatureDiedThisTurnOrMore(1)
            )
        ),
        "Feast of the Victorious Dead should be gated by one or more creatures dying this turn"
    );

    let effects = triggered.effects.flattened_default_effects();
    fn find_gain_life(effect: &crate::effect::Effect) -> Option<&GainLifeEffect> {
        if let Some(gain_life) = effect.downcast_ref::<GainLifeEffect>() {
            return Some(gain_life);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(find_gain_life);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return find_gain_life(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return find_gain_life(&with_id.effect);
        }
        None
    }
    fn find_put_counters(
        effect: &crate::effect::Effect,
    ) -> Option<&crate::effects::PutCountersEffect> {
        if let Some(put_counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
            return Some(put_counters);
        }
        if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
            return sequence.effects.iter().find_map(find_put_counters);
        }
        if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
            return find_put_counters(&tagged.effect);
        }
        if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
            return find_put_counters(&with_id.effect);
        }
        None
    }
    let gain_life = effects
        .iter()
        .find_map(|effect| find_gain_life(effect))
        .expect("Feast of the Victorious Dead should gain life");
    assert_eq!(
        gain_life.amount,
        Value::CreaturesDiedThisTurn,
        "Feast life gain should count creatures that died this turn"
    );

    let put_counters = effects
        .iter()
        .find_map(|effect| find_put_counters(effect))
        .expect("Feast of the Victorious Dead should distribute counters");
    assert_eq!(
        put_counters.amount,
        Value::CreaturesDiedThisTurn,
        "Feast counter distribution should count creatures that died this turn"
    );
    assert!(
        put_counters.distributed,
        "Feast should lower as a distributed counter effect"
    );
    assert!(
        put_counters.target_count.is_some(),
        "Feast should preserve the any-number distribution target count"
    );
    let (inner_target, target_count) = match &put_counters.target {
        ChooseSpec::WithCount(inner, count) => (inner.as_ref(), count),
        other => panic!("expected counted distribution target, got {other:?}"),
    };
    assert_eq!(
        target_count.min, 0,
        "Feast should allow zero distribution targets"
    );
    assert_eq!(
        target_count.max, None,
        "Feast should allow any number of distribution targets"
    );
    assert_eq!(
        put_counters.target_count,
        Some(*target_count),
        "Feast should preserve the same distribution target count on the effect"
    );
    let target_filter = match inner_target {
        ChooseSpec::Object(filter) => filter,
        other => panic!("expected object distribution target, got {other:?}"),
    };
    assert_eq!(
        target_filter.zone,
        Some(Zone::Battlefield),
        "Feast should distribute counters only on battlefield objects"
    );
    assert_eq!(
        target_filter.controller,
        Some(PlayerFilter::You),
        "Feast should distribute counters only among creatures you control"
    );
    assert!(
        target_filter.card_types.contains(&CardType::Creature),
        "Feast should distribute counters only among creatures, got {target_filter:?}"
    );

    assert!(
        rendered.contains(
            "At the beginning of your end step, if one or more creatures died this turn, you gain that much life and distribute that many +1/+1 counters among any number of creatures you control."
        ),
        "expected Feast compiled text to preserve the end-step died-this-turn distribution, got {rendered}"
    );
}

#[test]
pub(super) fn wondrous_crucible_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Wondrous Crucible");

    let def = parse_oracle_card_definition("Wondrous Crucible");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        (ability_debug.contains("GrantAbility")
            || ability_debug.contains("GrantObjectAbilityForFilter"))
            && ability_debug.contains("Ward")
            && ability_debug.contains("Generic(2)"),
        "Wondrous Crucible should grant ward {{2}} to permanents you control, got {ability_debug}"
    );
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Wondrous Crucible should have an end-step triggered ability");
    let effects = triggered.effects.flattened_default_effects();
    let exile = effects
        .iter()
        .find_map(|effect| {
            if let Some(exile) = effect.downcast_ref::<MoveToZoneEffect>() {
                return Some(exile);
            }
            effect
                .downcast_ref::<crate::effects::SequenceEffect>()?
                .effects
                .iter()
                .find_map(|nested| nested.downcast_ref::<MoveToZoneEffect>())
        })
        .expect("Wondrous Crucible should exile a card from the graveyard");
    let ChooseSpec::Object(filter) = exile.target.base() else {
        panic!(
            "Wondrous Crucible exile target should be a graveyard object filter, got {:?}",
            exile.target
        );
    };

    assert_eq!(exile.zone, Zone::Exile);
    assert!(
        exile.target.count().is_random(),
        "Wondrous Crucible should structurally mark the graveyard exile choice as random"
    );
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(filter.excluded_card_types.contains(&CardType::Land));
    assert!(
        ability_debug.contains("CastTaggedEffect")
            && ability_debug.contains("as_copy: true")
            && ability_debug.contains("without_paying_mana_cost: true"),
        "Wondrous Crucible should copy the exiled card and allow casting that copy for free, got {ability_debug}"
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("exile a nonland card at random from your graveyard"),
        "compiled text should preserve the at-random exile clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Copy it. You may cast a copy of the exiled card without paying its mana cost"
        ),
        "compiled text should compact the copy/cast-copy sequence, got {rendered}"
    );
}

#[test]
pub(super) fn capricious_hellraiser_preserves_random_tagged_graveyard_exile() {
    assert_oracle_card_parses_strict("Capricious Hellraiser");

    let def = parse_oracle_card_definition("Capricious Hellraiser");
    let rendered = compiled_text_lines(&def).join("\n");
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Capricious Hellraiser should have an enters triggered ability");
    let effects = triggered.effects.flattened_default_effects();
    let exile = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<TaggedEffect>()
                .and_then(|tagged| tagged.effect.downcast_ref::<crate::effects::ExileEffect>())
        })
        .expect("Capricious Hellraiser should tag its graveyard exile");
    let choose = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
        .expect("Capricious Hellraiser should choose from its exiled collection");

    assert_eq!(
        exile.spec.count(),
        ChoiceCount::exactly(3).at_random(),
        "Capricious Hellraiser should structurally retain its random three-card choice"
    );
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("exile three cards at random from your graveyard"),
        "compiled text should preserve the tagged random-exile clause, got {rendered}"
    );
    assert!(choose.filter.any_of.is_empty(), "{:#?}", choose.filter);
    assert_eq!(
        choose.filter.excluded_card_types,
        vec![CardType::Creature, CardType::Land]
    );
    assert!(
        rendered.contains(
            "Choose a noncreature, nonland card from among them and copy it. You may cast the copy without paying its mana cost"
        ),
        "compiled text should retain the exact conjunctive choice and copy permission, got {rendered}"
    );
}

#[test]
pub(super) fn choco_seeker_of_paradise_preserves_the_two_stage_looked_partition() {
    assert_oracle_card_parses_strict("Choco, Seeker of Paradise");
    let def = parse_oracle_card_definition("Choco, Seeker of Paradise");
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "look at that many cards from the top of your library. You may put one of them into your hand. Then put any number of land cards from among them onto the battlefield tapped and the rest into your graveyard"
        ),
        "Choco should preserve both selected groups and their exact remainder, got {rendered}"
    );
}

#[test]
pub(super) fn enshrined_memories_preserves_dynamic_reveal_partition() {
    assert_oracle_card_parses_strict("Enshrined Memories");
    let def = parse_oracle_card_definition("Enshrined Memories");
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "Reveal the top X cards of your library. Put all creature cards revealed this way into your hand and the rest on the bottom of your library in any order"
        ),
        "Enshrined Memories should keep the dynamic matching set and exact remainder, got {rendered}"
    );
}

#[test]
pub(super) fn marchesa_dealer_of_death_keeps_the_conditional_looked_partition() {
    assert_oracle_card_parses_strict("Marchesa, Dealer of Death");
    let def = parse_oracle_card_definition("Marchesa, Dealer of Death");
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "If you do, look at the top two cards of your library. Put one of them into your hand and the other into your graveyard"
        ),
        "Marchesa should retain both halves inside the payment-result branch, got {rendered}"
    );
}

#[test]
pub(super) fn tchaka_venerable_king_binds_the_inline_milled_collection() {
    assert_oracle_card_parses_strict("T'Chaka, Venerable King");
    let def = parse_oracle_card_definition("T'Chaka, Venerable King");
    let rendered = compiled_text_lines(&def).join("\n");

    assert!(
        rendered.contains("mill three cards")
            && rendered
                .contains("put an artifact or land card from among the milled cards into your hand"),
        "T'Chaka should keep the optional choice bound to the inline mill result, got {rendered}"
    );
}

#[test]
pub(super) fn become_anonymous_uses_one_hidden_pile_and_a_real_cloak_operation() {
    assert_oracle_card_parses_strict("Become Anonymous");
    let def = parse_oracle_card_definition("Become Anonymous");
    let rendered = compiled_text_lines(&def).join("\n");
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Become Anonymous should be a spell")
        .flattened_default_effects();
    let (pile_tag, targeted_exile_effect) = effects
        .iter()
        .find_map(|effect| {
            if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
                return Some((&tag_all.tag, tag_all.effect.as_ref()));
            }
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map(|tagged| (&tagged.tag, tagged.effect.as_ref()))
        })
        .expect("the targeted creature should establish the hidden pile tag");
    let library_exile = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>())
        .expect("the top two cards should be exiled without being looked at");
    let cloak = effects
        .iter()
        .find_map(|effect| {
            effect
                .downcast_ref::<crate::effects::ManifestObjectsEffect>()
                .or_else(|| {
                    effect
                        .downcast_ref::<crate::effects::TaggedEffect>()
                        .and_then(|tagged| {
                            tagged
                                .effect
                                .downcast_ref::<crate::effects::ManifestObjectsEffect>()
                        })
                })
                .or_else(|| {
                    effect
                        .downcast_ref::<crate::effects::TagAllEffect>()
                        .and_then(|tag_all| {
                            tag_all
                                .effect
                                .downcast_ref::<crate::effects::ManifestObjectsEffect>()
                        })
                })
        })
        .expect("the complete hidden pile should use the cloak runtime primitive");

    assert!(
        targeted_exile_effect
            .downcast_ref::<crate::effects::ExileEffect>()
            .or_else(|| {
                targeted_exile_effect
                    .downcast_ref::<crate::effects::TaggedEffect>()
                    .and_then(|tagged| tagged.effect.downcast_ref::<crate::effects::ExileEffect>())
            })
            .is_some_and(|exile| exile.face_down),
        "targeted pile member should be exiled face down"
    );
    assert!(library_exile.face_down, "library cards must stay hidden");
    assert_eq!(library_exile.count, Value::Fixed(2));
    assert_eq!(
        library_exile.accumulated_tags.as_slice(),
        [pile_tag.clone()],
        "top cards should append to, rather than replace, the targeted pile"
    );
    assert!(cloak.cloak && cloak.shuffle && cloak.tapped);
    assert_eq!(cloak.controller, PlayerFilter::You);
    assert!(matches!(
        cloak.target.base(),
        ChooseSpec::Tagged(tag) if tag == pile_tag
    ));
    assert!(
        rendered.contains(
            "Exile target nontoken creature you own and the top two cards of your library in a face-down pile, shuffle that pile, then cloak those cards. They enter tapped"
        ),
        "Become Anonymous should preserve the hidden pile, shuffle, cloak, and tapped entry, got {rendered}"
    );
}

#[test]
pub(super) fn consulate_surveillance_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Consulate Surveillance");
    let def = parse_oracle_card_definition("Consulate Surveillance");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Consulate Surveillance should parse its enters trigger strictly"
    );
    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Activated(_))),
        "Consulate Surveillance should parse its activated prevention ability strictly"
    );
    assert!(
        ability_debug.contains("EnergyCountersEffect")
            && ability_debug.contains("PreventAllDamageEffect")
            && ability_debug.contains("source_of_your_choice: true"),
        "expected energy trigger and chosen-source prevent-all-damage effect, got {ability_debug}"
    );
    assert!(
        rendered.contains("When this enchantment enters, you get {E}{E}{E}{E}")
            && rendered.contains(
                "Pay {E}{E}: Prevent all damage that would be dealt to you this turn by a source of your choice"
            ),
        "expected Consulate Surveillance compiled text to preserve energy and source-choice prevention clauses, got {rendered}"
    );
}

#[test]
pub(super) fn consulate_surveillance_activation_cost_requires_and_spends_two_energy() {
    let def = parse_oracle_card_definition("Consulate Surveillance");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Consulate Surveillance should have an activated ability");
    let costs = activated.mana_cost.costs();
    assert_eq!(
        costs.len(),
        1,
        "Consulate Surveillance activation should have only its energy payment cost"
    );
    assert!(
        costs[0].display().contains("{E}{E}"),
        "expected two-energy activation cost, got {}",
        costs[0].display()
    );

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.player_mut(alice)
        .expect("alice exists")
        .energy_counters = 1;
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let ctx = crate::costs::CostContext::new(source, alice, &mut dm);
    assert!(
        costs[0].can_pay(&game, &ctx).is_err(),
        "Consulate Surveillance activation should not be payable with one energy"
    );

    game.player_mut(alice)
        .expect("alice exists")
        .energy_counters = 2;
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::costs::CostContext::new(source, alice, &mut dm);
    costs[0]
        .pay(&mut game, &mut ctx)
        .expect("Consulate Surveillance activation should spend two energy");
    assert_eq!(
        game.player(alice).expect("alice exists").energy_counters,
        0,
        "Consulate Surveillance activation should spend exactly two energy"
    );
}

#[test]
pub(super) fn katara_seeking_revenge_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Katara, Seeking Revenge");
    let def = parse_oracle_card_definition("Katara, Seeking Revenge");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let raw = format!("{def:#?}");

    // ETB triggers now honor the oracle's self-naming surface ("When Katara
    // enters"); the waterbend cost, the unless-paid discard, and the "for each"
    // Lesson scaling all render faithfully to the oracle wording.
    assert!(
        rendered.contains("As an additional cost to cast this spell, you may waterbend {2}.")
            && rendered.contains(
                "When Katara enters, draw a card, then discard a card unless this spell's additional cost was paid.",
            )
            && rendered.contains("Katara gets +1/+1 for each Lesson card in your graveyard"),
        "Katara, Seeking Revenge compiled text should preserve waterbend, conditional discard, and Lesson scaling, got {rendered}"
    );
    assert!(
        raw.contains("ThisSpellPaidLabel")
            && raw.contains("Additional")
            && raw.contains("ConditionalEffect")
            && raw.contains("DiscardEffect")
            && raw.contains("waterbend_cost_2"),
        "Katara, Seeking Revenge should structurally lower waterbend and unless-paid discard, got {raw}"
    );
}

#[test]
pub(super) fn katara_seeking_revenge_waterbend_optional_cost_has_mana_and_tap_branches() {
    let def = parse_oracle_card_definition("Katara, Seeking Revenge");
    assert_eq!(
        def.optional_costs.len(),
        1,
        "Katara should have one optional waterbend cost"
    );
    assert!(
        def.optional_costs[0]
            .source_label
            .to_ascii_lowercase()
            .contains("waterbend {2}"),
        "Katara optional cost label should preserve waterbend, got {:?}",
        def.optional_costs[0].source_label
    );

    let branches = def.optional_costs[0]
        .cost
        .as_one_of()
        .expect("waterbend {2} should lower to alternative payment branches");
    assert_eq!(
        branches.len(),
        3,
        "waterbend {{2}} should have 0, 1, and 2 tap branches"
    );
    assert_eq!(
        branches[0].mana_cost().map(ManaCost::to_oracle),
        Some("{2}".to_string()),
        "first waterbend branch should be ordinary mana"
    );
    assert_eq!(
        branches[1].mana_cost().map(ManaCost::to_oracle),
        Some("{1}".to_string()),
        "second waterbend branch should require one remaining generic mana"
    );
    assert!(
        branches[2].mana_cost().is_none(),
        "third waterbend branch should be fully paid by tapping"
    );

    for (branch, expected_count) in [(&branches[1], 1), (&branches[2], 2)] {
        let choose = branch
            .costs()
            .iter()
            .filter_map(|cost| cost.effect_ref())
            .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
            .expect("waterbend tap branch should choose objects to tap");
        assert_eq!(choose.count.min, expected_count);
        assert_eq!(choose.count.max, Some(expected_count));
        assert!(choose.filter.untapped, "waterbend choices must be untapped");
        assert_eq!(choose.filter.controller, Some(PlayerFilter::You));
        assert!(
            choose
                .filter
                .any_of
                .iter()
                .any(|filter| filter.card_types.contains(&CardType::Artifact))
                && choose
                    .filter
                    .any_of
                    .iter()
                    .any(|filter| filter.card_types.contains(&CardType::Creature)),
            "waterbend choices should be artifacts or creatures, got {:?}",
            choose.filter
        );
    }
}

#[test]
pub(super) fn katara_seeking_revenge_waterbend_tap_cost_taps_chosen_artifact_and_creature() {
    struct ChooseFirstLegalObjects;

    impl crate::decision::DecisionMaker for ChooseFirstLegalObjects {
        fn decide_objects(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .iter()
                .filter(|candidate| candidate.legal)
                .map(|candidate| candidate.id)
                .take(ctx.max.unwrap_or(ctx.min) as usize)
                .collect()
        }
    }

    let def = parse_oracle_card_definition("Katara, Seeking Revenge");
    let tap_branch = &def.optional_costs[0]
        .cost
        .as_one_of()
        .expect("waterbend cost should have branches")[2];
    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let artifact = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Katara Waterbend Artifact")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );
    let creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Katara Waterbend Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let mut dm = ChooseFirstLegalObjects;
    let mut ctx = crate::costs::CostContext::new(source, alice, &mut dm);
    for cost in tap_branch.costs() {
        cost.pay(&mut game, &mut ctx)
            .expect("waterbend tap cost branch should be payable");
    }

    assert!(
        game.is_tapped(artifact),
        "waterbend should tap the chosen artifact"
    );
    assert!(
        game.is_tapped(creature),
        "waterbend should tap the chosen creature"
    );
}

#[test]
pub(super) fn katara_seeking_revenge_unpaid_additional_cost_discards_and_paid_cost_skips() {
    fn hand_card(game: &mut crate::game_state::GameState, player: PlayerId, name: &str) {
        let card = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_definition(&card, player, Zone::Hand);
    }

    fn etb_conditional(def: &CardDefinition) -> crate::effects::ConditionalEffect {
        fn find_conditional(
            effect: &crate::effect::Effect,
        ) -> Option<crate::effects::ConditionalEffect> {
            if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
                return Some(conditional.clone());
            }
            effect
                .downcast_ref::<crate::effects::SequenceEffect>()
                .and_then(|sequence| sequence.effects.iter().find_map(find_conditional))
        }

        def.abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => triggered
                    .effects
                    .flattened_default_effects()
                    .iter()
                    .find_map(find_conditional),
                _ => None,
            })
            .expect("Katara ETB should include an unless-paid conditional discard")
    }

    let def = parse_oracle_card_definition("Katara, Seeking Revenge");
    let conditional = etb_conditional(&def);

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    hand_card(&mut game, alice, "Katara Revenge Fodder");
    let mut ctx = crate::effects::ExecutionContext::new_default(katara, alice);
    conditional
        .execute(&mut game, &mut ctx)
        .expect("unpaid Katara discard branch should resolve");
    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 0);
    assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 1);

    let mut game = crate::game_state::GameState::new(vec!["Alice".to_string()], 20);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.object_mut(katara)
        .expect("Katara object exists")
        .optional_costs_paid
        .mark_label_paid(&def.optional_costs[0].source_label);
    hand_card(&mut game, alice, "Katara Revenge Kept Card");
    let mut ctx = crate::effects::ExecutionContext::new_default(katara, alice);
    conditional
        .execute(&mut game, &mut ctx)
        .expect("paid Katara discard branch should resolve");
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        1,
        "paying Katara's waterbend additional cost should skip the discard"
    );
    assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 0);
}

#[test]
pub(super) fn katara_seeking_revenge_counts_lesson_cards_in_its_controllers_graveyard() {
    let oracle_text = oracle_text_by_name()
        .get("Katara, Seeking Revenge")
        .expect("Katara oracle text should exist")
        .clone();
    let def = CardDefinitionBuilder::new(CardId::new(), "Katara, Seeking Revenge")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(oracle_text)
        .expect("Katara oracle text should parse for runtime P/T check");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let lesson = CardDefinitionBuilder::new(CardId::new(), "Katara Lesson")
        .card_types(vec![CardType::Sorcery])
        .subtypes(vec![Subtype::Lesson])
        .build();
    let non_lesson = CardDefinitionBuilder::new(CardId::new(), "Katara Non-Lesson")
        .card_types(vec![CardType::Sorcery])
        .build();

    let chars = game
        .calculated_characteristics(katara)
        .expect("Katara should have calculated characteristics");
    assert_eq!((chars.power, chars.toughness), (Some(3), Some(3)));

    game.create_object_from_definition(&lesson, alice, Zone::Graveyard);
    game.create_object_from_definition(&lesson, alice, Zone::Graveyard);
    game.create_object_from_definition(&lesson, bob, Zone::Graveyard);
    game.create_object_from_definition(&non_lesson, alice, Zone::Graveyard);

    let chars = game
        .calculated_characteristics(katara)
        .expect("Katara should have calculated characteristics");
    assert_eq!((chars.power, chars.toughness), (Some(5), Some(5)));
}

#[test]
pub(super) fn katara_waterbending_master_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Katara, Waterbending Master");
    let def = parse_oracle_card_definition("Katara, Waterbending Master");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(
        rendered,
        "Whenever you cast a spell during an opponent's turn, you get an experience counter.\n\
Whenever Katara attacks, you may draw a card for each experience counter you have. If you do, discard a card.",
        "Katara compiled text should preserve experience-counter and if-you-do discard clauses"
    );
    assert!(
        ability_debug.contains("ExperienceCountersEffect")
            && ability_debug.contains("PlayerCounters")
            && ability_debug.contains("Experience")
            && ability_debug.contains("MayEffect")
            && ability_debug.contains("IfEffect")
            && ability_debug.contains("DiscardEffect"),
        "Katara should structurally lower both triggers, got {ability_debug}"
    );
}

#[test]
pub(super) fn katara_experience_counter_effect_and_count_value_resolve_for_controller() {
    let def = parse_oracle_card_definition("Katara, Waterbending Master");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let mut ctx = crate::effects::ExecutionContext::new_default(katara, alice);

    crate::effects::ExperienceCountersEffect::new(1, PlayerFilter::You)
        .execute(&mut game, &mut ctx)
        .expect("Katara experience-counter effect should resolve");
    assert_eq!(
        game.player(alice)
            .expect("Alice exists")
            .experience_counters,
        1,
        "Katara controller should get one experience counter"
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").experience_counters,
        0,
        "opponent should not get Katara's experience counter"
    );

    game.player_mut(bob)
        .expect("Bob exists")
        .experience_counters = 3;
    let you_count = crate::effect::Value::PlayerCounters(
        PlayerFilter::You,
        crate::object::CounterType::Experience,
    );
    let opponent_count = crate::effect::Value::PlayerCounters(
        PlayerFilter::Opponent,
        crate::object::CounterType::Experience,
    );
    assert_eq!(
        crate::effects::helpers::resolve_value(&game, &you_count, &ctx)
            .expect("Katara's draw count should resolve from your experience counters"),
        1
    );
    assert_eq!(
        crate::effects::helpers::resolve_value(&game, &opponent_count, &ctx)
            .expect("opponent experience counter value should resolve independently"),
        3
    );
}

#[test]
pub(super) fn katara_spell_cast_trigger_only_fires_during_opponents_turn() {
    fn spell_cast_event(spell: ObjectId, caster: PlayerId) -> crate::triggers::TriggerEvent {
        crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        )
    }

    fn instant_definition(name: &str) -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build()
    }

    let def = parse_oracle_card_definition("Katara, Waterbending Master");
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.turn.active_player = alice;
    let own_turn_spell = game.create_object_from_definition(
        &instant_definition("Alice Own-Turn Instant"),
        alice,
        Zone::Stack,
    );
    assert!(
        crate::triggers::check_triggers(&game, &spell_cast_event(own_turn_spell, alice))
            .into_iter()
            .all(|entry| entry.source != katara),
        "Katara should not trigger when its controller casts a spell during their own turn"
    );

    game.turn.active_player = bob;
    let opponent_spell = game.create_object_from_definition(
        &instant_definition("Bob Opponent Instant"),
        bob,
        Zone::Stack,
    );
    assert!(
        crate::triggers::check_triggers(&game, &spell_cast_event(opponent_spell, bob))
            .into_iter()
            .all(|entry| entry.source != katara),
        "Katara should not trigger for an opponent casting a spell during that opponent's turn"
    );

    let opponents_turn_spell = game.create_object_from_definition(
        &instant_definition("Alice Opponent-Turn Instant"),
        alice,
        Zone::Stack,
    );
    let triggered =
        crate::triggers::check_triggers(&game, &spell_cast_event(opponents_turn_spell, alice));
    let entry = triggered
        .iter()
        .find(|entry| entry.source == katara)
        .expect(
            "Katara should trigger when its controller casts a spell during an opponent's turn",
        );
    assert_eq!(
        entry.ability.trigger.display(),
        "Whenever you cast a spell during an opponent's turn"
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(katara, alice, &mut dm)
        .with_triggering_event(entry.triggering_event.clone());
    for effect in &entry.ability.effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Katara opponent-turn spell trigger should resolve");
    }
    assert_eq!(
        game.player(alice)
            .expect("Alice exists")
            .experience_counters,
        1,
        "Katara's controller should get one experience counter from the trigger"
    );
    assert_eq!(
        game.player(bob).expect("Bob exists").experience_counters,
        0,
        "the opponent should not get Katara's experience counter"
    );
}

#[test]
pub(super) fn katara_if_you_do_discard_branch_discards_from_controller_only() {
    fn hand_card(
        game: &mut crate::game_state::GameState,
        player: PlayerId,
        name: &str,
    ) -> ObjectId {
        let card = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_definition(&card, player, Zone::Hand)
    }

    fn katara_attack_if_effect(def: &CardDefinition) -> crate::effects::IfEffect {
        def.abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered)
                    if {
                        let debug = format!("{:#?}", triggered.effects);
                        debug.contains("PlayerCounters") && debug.contains("DrawCardsEffect")
                    } =>
                {
                    triggered
                        .effects
                        .flattened_default_effects()
                        .iter()
                        .find_map(|effect| effect.downcast_ref::<IfEffect>().cloned())
                }
                _ => None,
            })
            .expect("Katara attack trigger should have an if-you-do follow-up")
    }

    let def = parse_oracle_card_definition("Katara, Waterbending Master");
    let if_effect = katara_attack_if_effect(&def);
    let discard = if_effect
        .then
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::DiscardEffect>())
        .expect("Katara if-you-do branch should discard a card");
    assert_eq!(
        discard.player,
        PlayerFilter::You,
        "Katara's implicit discard must bind to the controller, not the defending player"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    hand_card(&mut game, alice, "Katara Discard Fodder");
    hand_card(&mut game, bob, "Bob Should Keep This");
    let mut ctx = crate::effects::ExecutionContext::new_default(katara, alice);
    ctx.store_outcome(if_effect.condition, crate::effect::EffectOutcome::count(1));

    if_effect
        .execute(&mut game, &mut ctx)
        .expect("Katara positive if-you-do branch should resolve");
    assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 0);
    assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 1);
    assert_eq!(
        game.player(bob).expect("Bob exists").hand.len(),
        1,
        "defending player should not discard for Katara's if-you-do branch"
    );

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let katara = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    hand_card(&mut game, alice, "Katara Declined Fodder");
    let mut ctx = crate::effects::ExecutionContext::new_default(katara, alice);
    ctx.store_outcome(
        if_effect.condition,
        crate::effect::EffectOutcome::declined(),
    );

    if_effect
        .execute(&mut game, &mut ctx)
        .expect("Katara declined if-you-do branch should resolve without discarding");
    assert_eq!(
        game.player(alice).expect("Alice exists").hand.len(),
        1,
        "declining the draw should skip the discard branch"
    );
    assert_eq!(game.player(alice).expect("Alice exists").graveyard.len(), 0);
}

#[test]
pub(super) fn consulate_surveillance_prevents_damage_from_chosen_source_only() {
    struct ChooseSourceDecisionMaker {
        chosen: ObjectId,
    }

    impl crate::decision::DecisionMaker for ChooseSourceDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            if ctx
                .candidates
                .iter()
                .any(|candidate| candidate.id == self.chosen && candidate.legal)
            {
                vec![self.chosen]
            } else {
                Vec::new()
            }
        }
    }

    let def = parse_oracle_card_definition("Consulate Surveillance");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Consulate Surveillance should have an activated ability");
    let creature_def = CardDefinitionBuilder::new(CardId::new(), "Damage Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let chosen_source = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);
    let other_source = game.create_object_from_definition(&creature_def, bob, Zone::Battlefield);
    let surveillance = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let mut dm = ChooseSourceDecisionMaker {
        chosen: chosen_source,
    };
    let mut ctx = crate::effects::ExecutionContext::new(surveillance, alice, &mut dm);
    for effect in activated.effects.flattened_default_effects() {
        effect
            .0
            .execute(&mut game, &mut ctx)
            .expect("Consulate Surveillance prevention effect should resolve");
    }

    let shields = game.effect_store.prevention_effects.shields();
    assert_eq!(
        shields.len(),
        1,
        "activation should create one prevention shield"
    );
    assert_eq!(
        shields[0].damage_filter.from_specific_source,
        Some(chosen_source),
        "prevention shield should be restricted to the chosen source"
    );

    let (chosen_remaining, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        chosen_source,
        crate::events::DamageTarget::Player(alice),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        chosen_remaining, 0,
        "chosen source damage should be prevented"
    );

    let (chosen_to_bob_remaining, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        chosen_source,
        crate::events::DamageTarget::Player(bob),
        3,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        chosen_to_bob_remaining, 3,
        "chosen source damage to another player should not be prevented"
    );

    let (other_remaining, _) = crate::events::processing::process_damage_with_event(
        &mut game,
        other_source,
        crate::events::DamageTarget::Player(alice),
        2,
        false,
        crate::events::cause::EventCause::effect(),
    );
    assert_eq!(
        other_remaining, 2,
        "damage from a different source should not be prevented"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Staff of the Storyteller");
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let ability_debug = format!("{:#?}", def.abilities);
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join("\n");
    let oracle = oracle_text_by_name()
        .get("Staff of the Storyteller")
        .expect("Staff of the Storyteller oracle text")
        .clone();
    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            &oracle,
            &compiled,
            crate::semantic_compare::report_embedding_config(),
        );

    assert!(
        ability_debug.contains("TokensCreated")
            && ability_debug.contains("PutCountersEffect")
            && ability_debug.contains("story")
            && ability_debug.contains("RemoveCountersEffect")
            && ability_debug.contains("DrawCardsEffect"),
        "Staff should structurally keep token-created trigger, story counter, and draw activation, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "Whenever you create one or more creature tokens, put a story counter on this artifact."
        ) && rendered.contains("{W}, {T}, Remove a story counter from this artifact: Draw a card."),
        "expected Staff compiled text to preserve token-created trigger and activation, got {rendered}"
    );
    assert!(
        similarity >= 0.99 && !mismatch,
        "expected Staff semantic comparison to clear target, score={similarity}, mismatch={mismatch}, compiled={compiled:?}"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_enters_token_adds_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let enters = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::zones::ZoneChangeEvent::with_cause(
            staff,
            Zone::Hand,
            Zone::Battlefield,
            crate::events::cause::EventCause::effect(),
            None,
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert_eq!(
        resolve_triggers_for_source(&mut game, staff, &enters),
        1,
        "Staff should trigger once when it enters"
    );

    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let staff_token_triggers = trigger_queue
        .entries
        .iter()
        .filter(|entry| entry.source == staff)
        .count();
    assert_eq!(
        staff_token_triggers, 1,
        "Staff should trigger once from the Spirit creature token it created"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Staff token-created trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Staff token-created trigger should resolve");

    let spirit_tokens = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|object| {
            matches!(object.kind, crate::object::ObjectKind::Token)
                && object.subtypes.contains(&Subtype::Spirit)
                && game.controller_of(object) == alice
        })
        .count();
    assert_eq!(spirit_tokens, 1, "Staff should create one Spirit token");
    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        1,
        "Staff should get a story counter from creating a creature token"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_noncreature_token_does_not_add_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let clue = CardDefinitionBuilder::new(CardId::new(), "Clue")
        .token()
        .card_types(vec![CardType::Artifact])
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let effect = crate::effect::Effect::new(CreateTokenEffect::you(clue, 1));
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice);

    crate::effects::execute_effect(&mut game, &effect, &mut ctx)
        .expect("creating a Clue token should resolve");
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue
            .entries
            .iter()
            .all(|entry| entry.source != staff),
        "Staff should not trigger from creating a noncreature token"
    );
    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        0,
        "noncreature tokens should not add story counters"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_multiple_creature_tokens_add_one_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let soldier = CardDefinitionBuilder::new(CardId::new(), "Soldier")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let effect = crate::effect::Effect::new(CreateTokenEffect::you(soldier, 2));
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice);

    crate::effects::execute_effect(&mut game, &effect, &mut ctx)
        .expect("creating two Soldier tokens should resolve");
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let staff_trigger_count = trigger_queue
        .entries
        .iter()
        .filter(|entry| entry.source == staff)
        .count();
    assert_eq!(
        staff_trigger_count, 1,
        "one-or-more token-created trigger should fire once for a batch of creature tokens"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Staff token-created trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Staff token-created trigger should resolve");

    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        1,
        "creating multiple creature tokens in one event should add one story counter"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_creature_token_copies_add_one_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let creature = CardDefinitionBuilder::new(CardId::new(), "Story Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let effect = CreateTokenCopyEffect::new(ChooseSpec::creature(), 2, PlayerFilter::You);
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);

    effect
        .execute(&mut game, &mut ctx)
        .expect("creating two creature token copies should resolve");
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);
    let staff_trigger_count = trigger_queue
        .entries
        .iter()
        .filter(|entry| entry.source == staff)
        .count();
    assert_eq!(
        staff_trigger_count, 1,
        "one-or-more token-created trigger should fire once for a batch of creature token copies"
    );
    crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
        .expect("Staff token-copy-created trigger should go on the stack");
    crate::game_loop::resolve_stack_entry(&mut game)
        .expect("Staff token-copy-created trigger should resolve");

    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        1,
        "creating multiple creature token copies in one event should add one story counter"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_noncreature_token_copy_does_not_add_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let artifact = CardDefinitionBuilder::new(CardId::new(), "Story Rock")
        .card_types(vec![CardType::Artifact])
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(&artifact, alice, Zone::Battlefield);
    let effect = CreateTokenCopyEffect::new(
        ChooseSpec::Object(crate::target::ObjectFilter::artifact()),
        1,
        PlayerFilter::You,
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);

    effect
        .execute(&mut game, &mut ctx)
        .expect("creating a noncreature token copy should resolve");
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue
            .entries
            .iter()
            .all(|entry| entry.source != staff),
        "Staff should not trigger from creating a noncreature token copy"
    );
    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        0,
        "noncreature token copies should not add story counters"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_opponent_creature_token_copy_does_not_add_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let creature = CardDefinitionBuilder::new(CardId::new(), "Opponent Story Bear")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Bear])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let effect = CreateTokenCopyEffect::new(ChooseSpec::creature(), 1, PlayerFilter::Opponent);
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);

    effect
        .execute(&mut game, &mut ctx)
        .expect("creating an opponent creature token copy should resolve");
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue
            .entries
            .iter()
            .all(|entry| entry.source != staff),
        "Staff should not trigger from creature token copies created by an opponent"
    );
    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        0,
        "opponent-created creature token copies should not add story counters"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_opponent_creature_token_does_not_add_story_counter() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let soldier = CardDefinitionBuilder::new(CardId::new(), "Soldier")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Soldier])
        .build();
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let effect =
        crate::effect::Effect::new(CreateTokenEffect::new(soldier, 1, PlayerFilter::Opponent));
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice);

    crate::effects::execute_effect(&mut game, &effect, &mut ctx)
        .expect("creating an opponent Soldier token should resolve");
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    crate::game_loop::drain_pending_trigger_events(&mut game, &mut trigger_queue);

    assert!(
        trigger_queue
            .entries
            .iter()
            .all(|entry| entry.source != staff),
        "Staff should not trigger from creature tokens created by an opponent"
    );
    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        0,
        "opponent-created creature tokens should not add story counters"
    );
}

#[test]
pub(super) fn staff_of_the_storyteller_activation_removes_story_counter_and_draws() {
    let def = parse_oracle_card_definition("Staff of the Storyteller");
    let activated = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Staff should have a draw activated ability");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let staff = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let top_card = CardDefinitionBuilder::new(CardId::new(), "Staff Drawn Card")
        .card_types(vec![CardType::Sorcery])
        .build();
    game.create_object_from_definition(&top_card, alice, Zone::Library);
    game.remove_summoning_sickness(staff);

    assert!(
        crate::cost::can_pay_cost(&game, staff, alice, &activated.mana_cost).is_err(),
        "Staff activation should not be payable without a story counter"
    );
    game.add_counters(staff, CounterType::Named("story"), 1)
        .expect("Staff should accept a story counter");
    assert!(
        crate::cost::can_pay_cost(&game, staff, alice, &activated.mana_cost).is_err(),
        "Staff activation should not be payable without white mana even with a story counter"
    );
    game.player_mut(alice)
        .expect("Alice should exist")
        .mana_pool
        .add(ManaSymbol::White, 1);
    crate::cost::can_pay_cost(&game, staff, alice, &activated.mana_cost)
        .expect("Staff activation should be payable with white mana, tap, and a story counter");
    let mut dm = crate::decision::AutoPassDecisionMaker::default();
    crate::special_actions::pay_total_cost_with_choice(
        &mut game,
        alice,
        staff,
        &activated.mana_cost,
        crate::costs::PaymentReason::ActivateAbility,
        &mut dm,
    )
    .expect("Staff activation cost should be paid");

    assert!(game.is_tapped(staff), "activation cost should tap Staff");
    assert_eq!(
        game.counter_count(staff, CounterType::Named("story")),
        0,
        "activation cost should remove the story counter"
    );
    let mut ctx = crate::effects::ExecutionContext::new_default(staff, alice);
    for effect in activated.effects.flattened_default_effects() {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Staff draw effect should resolve");
    }

    let hand_cards = game
        .objects_in_zone(Zone::Hand)
        .into_iter()
        .filter_map(|id| game.object(id))
        .filter(|object| object.name == "Staff Drawn Card")
        .count();
    assert_eq!(hand_cards, 1, "Staff activation should draw one card");
}

#[test]
pub(super) fn hydra_omnivore_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Hydra Omnivore");

    let def = parse_oracle_card_definition("Hydra Omnivore");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(def.name(), "Hydra Omnivore");
    assert!(
        ability_debug.contains("ThisDealsCombatDamageToPlayerTrigger")
            && ability_debug.contains("player: Opponent")
            && ability_debug.contains("ForPlayersEffect")
            && ability_debug.contains("Excluding")
            && ability_debug.contains("Opponent")
            && ability_debug.contains("DamagedPlayer")
            && ability_debug.contains("DealDamageEffect")
            && ability_debug.contains("EventValue")
            && ability_debug.contains("Amount"),
        "Hydra Omnivore should compile to damage each opponent except the damaged player, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "Whenever this creature deals combat damage to an opponent, it deals that much damage to each other opponent."
        ),
        "Hydra Omnivore should render the each-other-opponent damage clause, got {rendered}"
    );
}

#[test]
pub(super) fn hydra_omnivore_runtime_damages_each_other_opponent_only() {
    let def = parse_oracle_card_definition("Hydra Omnivore");
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dana".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let dana = PlayerId::from_index(3);
    let hydra = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let noncombat = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            hydra,
            crate::events::DamageTarget::Player(bob),
            8,
            false,
            crate::events::cause::EventCause::effect(),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, hydra, &noncombat),
        0,
        "Hydra Omnivore should not trigger from noncombat damage"
    );

    let hits_controller = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            hydra,
            crate::events::DamageTarget::Player(alice),
            8,
            true,
            crate::events::cause::EventCause::combat_damage(hydra),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, hydra, &hits_controller),
        0,
        "Hydra Omnivore should not trigger from combat damage to its controller"
    );

    let combat = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            hydra,
            crate::events::DamageTarget::Player(bob),
            8,
            true,
            crate::events::cause::EventCause::combat_damage(hydra),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, hydra, &combat),
        1,
        "Hydra Omnivore should trigger once from its combat damage to an opponent"
    );

    assert_eq!(game.life_total(alice), 20, "controller is not an opponent");
    assert_eq!(
        game.life_total(bob),
        20,
        "the damaged opponent should be excluded from each other opponent"
    );
    assert_eq!(
        game.life_total(charlie),
        12,
        "first other opponent should take 8"
    );
    assert_eq!(
        game.life_total(dana),
        12,
        "second other opponent should take 8"
    );
}

#[test]
pub(super) fn hydra_omnivore_runtime_has_no_extra_damage_without_other_opponents() {
    let def = parse_oracle_card_definition("Hydra Omnivore");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let hydra = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let combat = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            hydra,
            crate::events::DamageTarget::Player(bob),
            8,
            true,
            crate::events::cause::EventCause::combat_damage(hydra),
        ),
        crate::provenance::ProvNodeId::default(),
    );

    assert_eq!(resolve_triggers_for_source(&mut game, hydra, &combat), 1);
    assert_eq!(
        game.life_total(alice),
        20,
        "controller should not be damaged"
    );
    assert_eq!(
        game.life_total(bob),
        20,
        "with no other opponent, the damaged opponent should not take extra damage"
    );
}

pub(super) fn arvinox_the_mind_flail_definition() -> CardDefinition {
    let oracle = oracle_text_by_name()
        .get("Arvinox, the Mind Flail")
        .expect("Arvinox oracle text should be present");
    CardDefinitionBuilder::new(CardId::new(), "Arvinox, the Mind Flail")
        .card_types(vec![CardType::Enchantment, CardType::Creature])
        .subtypes(vec![Subtype::Horror])
        .power_toughness(PowerToughness::fixed(9, 9))
        .parse_text(oracle)
        .expect("Arvinox should parse strictly")
}

#[test]
pub(super) fn arvinox_the_mind_flail_strict_parser_and_compiled_text_regression() {
    let def = arvinox_the_mind_flail_definition();

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .contains("isn't a creature unless you control three or more permanents you don't own"),
        "expected unless static condition, got {rendered}"
    );
    assert!(
        rendered.contains("exile the bottom card of each opponent's library face down"),
        "expected bottom-card face-down exile, got {rendered}"
    );
    assert!(
        rendered.contains("Look at a card you control in exile, then you may cast that card for as long as it remains exiled"),
        "expected look-and-cast permission for the exiled card, got {rendered}"
    );
    assert!(
        rendered.contains("you may cast that card"),
        "expected cast permission for the exiled card, got {rendered}"
    );
    assert!(
        rendered
            .contains("you may spend mana as though it were mana of any color to cast that spell"),
        "expected any-color mana permission, got {rendered}"
    );

    let debug = format!("{def:#?}");
    assert!(debug.contains("bottom_only: true"), "{debug}");
    assert!(debug.contains("GrantPlayTaggedEffect"), "{debug}");
    assert!(debug.contains("filter: Some"), "{debug}");
}

#[test]
pub(super) fn arvinox_the_mind_flail_creature_condition_counts_permanents_you_control_but_dont_own()
{
    let def = arvinox_the_mind_flail_definition();
    let permanent = CardDefinitionBuilder::new(CardId::new(), "Borrowed Permanent")
        .card_types(vec![CardType::Artifact])
        .build();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let arvinox = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    assert!(
        !game.current_is_creature(arvinox),
        "Arvinox should not be a creature before Alice controls three permanents she doesn't own"
    );

    for idx in 0..2 {
        let borrowed = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
        game.set_current_controller(borrowed, alice);
        assert!(
            !game.current_is_creature(arvinox),
            "Arvinox should still not be a creature with only {} borrowed permanents",
            idx + 1
        );
    }

    let third = game.create_object_from_definition(&permanent, bob, Zone::Battlefield);
    game.set_current_controller(third, alice);

    assert!(
        game.current_is_creature(arvinox),
        "Arvinox should become a creature once Alice controls three permanents she doesn't own"
    );
}

#[test]
pub(super) fn arvinox_the_mind_flail_exiles_bottom_cards_and_grants_only_permanent_spell_permission()
 {
    let def = arvinox_the_mind_flail_definition();
    let permanent = CardDefinitionBuilder::new(CardId::new(), "Bottom Permanent")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let instant = CardDefinitionBuilder::new(CardId::new(), "Bottom Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let filler = CardDefinitionBuilder::new(CardId::new(), "Top Filler")
        .card_types(vec![CardType::Sorcery])
        .build();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let arvinox = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let bob_bottom = game.create_object_from_definition(&permanent, bob, Zone::Library);
    let bob_top = game.create_object_from_definition(&filler, bob, Zone::Library);
    let charlie_bottom = game.create_object_from_definition(&instant, charlie, Zone::Library);
    let charlie_top = game.create_object_from_definition(&filler, charlie, Zone::Library);
    let bob_bottom_stable = game.object(bob_bottom).expect("bob bottom setup").stable_id;
    let charlie_bottom_stable = game
        .object(charlie_bottom)
        .expect("charlie bottom setup")
        .stable_id;
    assert!(game.set_player_library_order_with_audit(
        bob,
        vec![bob_bottom, bob_top],
        "Arvinox bottom-card regression setup",
    ));
    assert!(game.set_player_library_order_with_audit(
        charlie,
        vec![charlie_bottom, charlie_top],
        "Arvinox bottom-card regression setup",
    ));

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Arvinox should have an end-step triggered ability");
    let mut ctx = crate::effects::ExecutionContext::new_default(arvinox, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        arvinox,
        &triggered.effects,
        None,
        &[],
    )
    .expect("Arvinox end-step trigger should resolve");

    let bob_exiled = game
        .find_object_by_stable_id(bob_bottom_stable)
        .expect("bob bottom should still exist after exile");
    let charlie_exiled = game
        .find_object_by_stable_id(charlie_bottom_stable)
        .expect("charlie bottom should still exist after exile");
    assert_eq!(
        game.object(bob_exiled).expect("bob bottom").zone,
        Zone::Exile
    );
    assert_eq!(game.object(bob_top).expect("bob top").zone, Zone::Library);
    assert_eq!(
        game.object(charlie_exiled).expect("charlie bottom").zone,
        Zone::Exile
    );
    assert_eq!(
        game.object(charlie_top).expect("charlie top").zone,
        Zone::Library
    );
    assert!(game.is_face_down(bob_exiled));
    assert!(game.is_face_down(charlie_exiled));
    assert!(
        game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            bob_exiled,
            Zone::Exile,
            alice,
        ),
        "Arvinox should grant Alice permission to cast exiled permanent spells"
    );
    assert!(
        !game.effect_store.grant_registry.card_can_play_from_zone(
            &game,
            charlie_exiled,
            Zone::Exile,
            alice,
        ),
        "Arvinox should not grant Alice permission to cast exiled nonpermanent spells"
    );
}

#[test]
pub(super) fn stolen_strategy_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Stolen Strategy");

    let def = parse_oracle_card_definition("Stolen Strategy");
    let rendered = compiled_text_lines(&def).join(" ");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(def.name(), "Stolen Strategy");
    assert_eq!(def.card.card_types, vec![CardType::Enchantment]);
    assert!(
        rendered.contains(
            "At the beginning of your upkeep, each opponent exiles the top card of their library. Until end of turn, you may cast spells from among those exiled cards, and you may spend mana as though it were mana of any color to cast those spells",
        ),
        "expected Stolen Strategy compiled text to preserve the per-opponent exile and tagged cast permission, got {rendered}"
    );
    assert!(
        ability_debug.contains("BeginningOfUpkeepTrigger")
            && ability_debug.contains("ForPlayersEffect")
            && ability_debug.contains("filter: Opponent")
            && ability_debug.contains("ExileTopOfLibraryEffect")
            && ability_debug.contains("player: IteratedPlayer")
            && ability_debug.contains("GrantPlayTaggedEffect")
            && ability_debug.contains("duration: UntilEndOfTurn")
            && ability_debug.contains("allow_any_color_for_cast: true"),
        "expected Stolen Strategy to lower to upkeep-triggered per-opponent top exile plus tagged cast permission, got {ability_debug}"
    );
}

#[test]
pub(super) fn stolen_strategy_runtime_exiles_each_opponents_top_card_and_grants_cast_permission() {
    let def = parse_oracle_card_definition("Stolen Strategy");
    let spell = CardDefinitionBuilder::new(CardId::new(), "Opponent Library Spell")
        .card_types(vec![CardType::Instant])
        .with_spell_effect(vec![Effect::draw(1)])
        .build();
    let filler = CardDefinitionBuilder::new(CardId::new(), "Library Filler")
        .card_types(vec![CardType::Sorcery])
        .with_spell_effect(vec![Effect::draw(1)])
        .build();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let stolen_strategy = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let alice_top = game.create_object_from_definition(&spell, alice, Zone::Library);
    let bob_bottom = game.create_object_from_definition(&filler, bob, Zone::Library);
    let bob_top = game.create_object_from_definition(&spell, bob, Zone::Library);
    let charlie_bottom = game.create_object_from_definition(&filler, charlie, Zone::Library);
    let charlie_top = game.create_object_from_definition(&spell, charlie, Zone::Library);
    let bob_top_stable = game.object(bob_top).expect("bob top setup").stable_id;
    let charlie_top_stable = game
        .object(charlie_top)
        .expect("charlie top setup")
        .stable_id;
    assert!(game.set_player_library_order_with_audit(
        bob,
        vec![bob_bottom, bob_top],
        "Stolen Strategy top-card regression setup",
    ));
    assert!(game.set_player_library_order_with_audit(
        charlie,
        vec![charlie_bottom, charlie_top],
        "Stolen Strategy top-card regression setup",
    ));

    let bob_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(bob),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, stolen_strategy, &bob_upkeep),
        0,
        "Stolen Strategy should not trigger on an opponent's upkeep"
    );
    assert_eq!(
        game.object(bob_top).expect("bob top before upkeep").zone,
        Zone::Library
    );
    assert_eq!(
        game.object(charlie_top)
            .expect("charlie top before upkeep")
            .zone,
        Zone::Library
    );

    let alice_upkeep = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::phase::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    assert_eq!(
        resolve_triggers_for_source(&mut game, stolen_strategy, &alice_upkeep),
        1,
        "Stolen Strategy should trigger at the beginning of its controller's upkeep"
    );

    let bob_exiled = game
        .find_object_by_stable_id(bob_top_stable)
        .expect("bob top card should still exist after exile");
    let charlie_exiled = game
        .find_object_by_stable_id(charlie_top_stable)
        .expect("charlie top card should still exist after exile");
    assert_eq!(
        game.object(alice_top).expect("alice top").zone,
        Zone::Library
    );
    assert_eq!(
        game.object(bob_bottom).expect("bob bottom").zone,
        Zone::Library
    );
    assert_eq!(
        game.object(charlie_bottom).expect("charlie bottom").zone,
        Zone::Library
    );
    assert_eq!(
        game.object(bob_exiled).expect("bob exiled").zone,
        Zone::Exile
    );
    assert_eq!(
        game.object(charlie_exiled).expect("charlie exiled").zone,
        Zone::Exile
    );

    for exiled in [bob_exiled, charlie_exiled] {
        assert!(
            game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled,
                Zone::Exile,
                alice,
            ),
            "Alice should be able to cast each card exiled by Stolen Strategy"
        );
        assert!(
            game.can_spend_mana_as_any_color(alice, Some(exiled)),
            "Alice should be able to spend mana as any color to cast each Stolen Strategy card"
        );
        assert!(
            !game.effect_store.grant_registry.card_can_play_from_zone(
                &game,
                exiled,
                Zone::Exile,
                bob,
            ),
            "Stolen Strategy should grant the cast permission only to its controller"
        );
    }
}

#[test]
pub(super) fn clockspinning_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Clockspinning");

    let def = parse_oracle_card_definition("Clockspinning");
    let spell_debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(def.name(), "Clockspinning");
    assert_eq!(def.card.card_types, vec![CardType::Instant]);
    assert!(
        def.optional_costs.iter().any(|cost| {
            cost.source_label == "Buyback"
                && cost.returns_to_hand
                && format!("{:?}", cost.cost).contains("Generic(3)")
        }),
        "Clockspinning should preserve buyback {{3}}, got {:?}",
        def.optional_costs
    );
    assert!(
        spell_debug.contains("ForEachCounterKindPutOrRemoveEffect")
            && spell_debug.contains("all_kinds: false")
            && spell_debug.contains("alternative_cast: Some")
            && spell_debug.contains("Suspend")
            && spell_debug.contains("with_counter: Some")
            && spell_debug.contains("Time"),
        "Clockspinning should compile to a one-counter-kind put/remove effect targeting permanents or suspended cards, got {spell_debug}"
    );
    assert!(
        rendered.contains(
            "Choose a counter on target permanent or suspended card. Remove that counter from that permanent or card or put another of those counters on it."
        ),
        "Clockspinning should render the chosen-counter put/remove clause, got {rendered}"
    );
}

#[test]
pub(super) fn rift_elemental_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Rift Elemental");

    let def = parse_oracle_card_definition("Rift Elemental");
    let activated_debug = format!("{:?}", def.abilities);
    let activated_pretty = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        activated_debug.contains("RemoveAnyCountersAmongEffect")
            && activated_debug.contains("alternative_cast: Some")
            && activated_debug.contains("Suspend")
            && activated_debug.contains("owner: Some(You)")
            && activated_debug.contains("counter_type: Some(Time)"),
        "Rift Elemental should preserve permanent-or-suspended-card time-counter cost model, got {activated_pretty}"
    );
    assert!(
        rendered.contains(
            "{1}{R}, Remove a time counter from a permanent you control or suspended card you own: This creature gets +2/+0 until end of turn."
        ),
        "Rift Elemental should render its full time-counter cost and self-reference, got {rendered}"
    );
}

#[test]
pub(super) fn all_of_history_all_at_once_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("All of History, All at Once");

    let def = parse_oracle_card_definition("All of History, All at Once");
    let spell_debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert_eq!(def.name(), "All of History, All at Once");
    assert!(
        rendered.contains("Time travel"),
        "All of History, All at Once should render its time travel keyword action, got {rendered}"
    );
    assert!(
        rendered.contains("Storm"),
        "All of History, All at Once should retain storm after time travel parses, got {rendered}"
    );
    assert!(
        spell_debug.contains("ForEachCounterKindPutOrRemoveEffect")
            && spell_debug.contains("fixed_counter_type: Some")
            && spell_debug.contains("Time")
            && spell_debug.contains("all_kinds: false")
            && spell_debug.contains("optional_action: true"),
        "All of History, All at Once should lower time travel to fixed time-counter put/remove support, got {spell_debug}"
    );
}

pub(super) fn ichormoon_gauntlet_chosen_counter_effect(def: &CardDefinition) -> &Effect {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .flattened_default_effects()
                .into_iter()
                .find(|effect| {
                    effect
                        .downcast_ref::<crate::effects::PutCounterOfChosenKindEffect>()
                        .is_some()
                }),
            _ => None,
        })
        .expect("Ichormoon Gauntlet should have a chosen-counter trigger effect")
}

#[test]
pub(super) fn ichormoon_gauntlet_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Ichormoon Gauntlet");

    let def = parse_oracle_card_definition("Ichormoon Gauntlet");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        ability_debug.contains("GrantObjectAbilityForFilter")
            && ability_debug.contains("ProliferateEffect")
            && ability_debug.contains("ExtraTurnEffect")
            && ability_debug.contains("RemoveCountersEffect")
            && ability_debug.contains("Loyalty")
            && ability_debug.contains("SpellCastTrigger")
            && ability_debug.contains("caster: You")
            && ability_debug.contains("excluded_card_types")
            && ability_debug.contains("Creature")
            && ability_debug.contains("PutCounterOfChosenKindEffect"),
        "Ichormoon Gauntlet should structurally grant loyalty abilities and preserve its noncreature-spell trigger, got {ability_debug}"
    );
    assert!(
        !ability_debug
            .contains("Named(\n                                                \"additional\"")
            && !ability_debug.contains("Named(\"additional\")"),
        "additional-counter wording should not lower to a named 'additional' counter, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "Planeswalkers you control have \"[0]: Proliferate\" and \"[−12]: Take an extra turn after this one.\""
        ),
        "expected compiled text to preserve granted bracketed loyalty abilities as one quoted line, got {rendered}"
    );
    assert!(
        rendered.contains("choose a counter on target permanent")
            || rendered.contains("Choose a counter on target permanent"),
        "expected compiled text to preserve choose-counter target clause, got {rendered}"
    );
    assert!(
        rendered.contains("Put an additional counter of that kind on that permanent"),
        "expected compiled text to preserve additional-counter-kind clause, got {rendered}"
    );
    assert!(
        !crate::cards::generated_definition_has_unimplemented_content(&def),
        "Ichormoon Gauntlet should compile without unsupported runtime markers: {ability_debug}"
    );
}

#[test]
pub(super) fn ichormoon_gauntlet_grants_two_loyalty_abilities_to_planeswalkers_you_control() {
    let def = parse_oracle_card_definition("Ichormoon Gauntlet");
    let planeswalker = CardDefinitionBuilder::new(CardId::new(), "Gauntlet Test Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .build();
    let creature = CardDefinitionBuilder::new(CardId::new(), "Gauntlet Test Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let alice_planeswalker =
        game.create_object_from_definition(&planeswalker, alice, Zone::Battlefield);
    let bob_planeswalker =
        game.create_object_from_definition(&planeswalker, bob, Zone::Battlefield);
    let alice_creature = game.create_object_from_definition(&creature, alice, Zone::Battlefield);

    let granted = game
        .current_abilities(alice_planeswalker)
        .expect("Alice planeswalker should have current abilities")
        .into_iter()
        .filter_map(|ability| match ability.kind {
            AbilityKind::Activated(activated) if activated.is_loyalty_ability => Some(activated),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        granted.len(),
        2,
        "controlled planeswalker should gain both Ichormoon loyalty abilities, got {granted:#?}"
    );
    assert!(
        granted.iter().any(|activated| activated.mana_cost.is_free()
            && activated
                .effects
                .flattened_default_effects()
                .into_iter()
                .any(|effect| effect
                    .downcast_ref::<crate::effects::ProliferateEffect>()
                    .is_some())),
        "expected granted [0] proliferate loyalty ability, got {granted:#?}"
    );
    assert!(
        granted.iter().any(
            |activated| activated.mana_cost.has_loyalty_activation_cost()
                && activated
                    .effects
                    .flattened_default_effects()
                    .into_iter()
                    .any(|effect| effect
                        .downcast_ref::<crate::effects::ExtraTurnEffect>()
                        .is_some())
        ),
        "expected granted -12 extra-turn loyalty ability, got {granted:#?}"
    );

    let bob_granted = game
        .current_abilities(bob_planeswalker)
        .expect("Bob planeswalker should have current abilities")
        .into_iter()
        .filter(|ability| {
            matches!(ability.kind, AbilityKind::Activated(ref activated) if activated.is_loyalty_ability)
        })
        .count();
    assert_eq!(
        bob_granted, 0,
        "opposing planeswalkers should not gain the abilities"
    );
    let creature_granted = game
        .current_abilities(alice_creature)
        .expect("Alice creature should have current abilities")
        .into_iter()
        .filter(|ability| {
            matches!(ability.kind, AbilityKind::Activated(ref activated) if activated.is_loyalty_ability)
        })
        .count();
    assert_eq!(
        creature_granted, 0,
        "non-planeswalkers should not gain the abilities"
    );
}

#[test]
pub(super) fn nicol_bolas_dragon_god_strict_parser_compiled_text_and_model_regression() {
    assert_oracle_card_parses_strict("Nicol Bolas, Dragon-God");

    let def = parse_oracle_card_definition("Nicol Bolas, Dragon-God");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        ability_debug.contains("CopyActivatedAbilities")
            && ability_debug.contains("only_loyalty: true")
            && ability_debug.contains("exclude_source_id: true"),
        "Nicol Bolas should structurally copy only loyalty abilities from other planeswalkers, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("ChooseObjectsEffect")
            && ability_debug.contains("zone: Some")
            && ability_debug.contains("Hand")
            && ability_debug.contains("additional_zones")
            && ability_debug.contains("Battlefield"),
        "Nicol Bolas +1 should structurally choose from hand or battlefield, got {ability_debug}"
    );
    assert!(
        ability_debug.contains("Not(")
            && ability_debug.contains("PlayerControls")
            && ability_debug.contains("LoseTheGameEffect"),
        "Nicol Bolas -8 should structurally require opponents without legendary creature/planeswalker to lose, got {ability_debug}"
    );
    assert!(
        rendered.contains(
            "Nicol Bolas has all loyalty abilities of all other planeswalkers on the battlefield"
        ),
        "expected compiled text to preserve loyalty-copy clause, got {rendered}"
    );
    assert!(
        rendered.contains(
            "+1: You draw a card. Each opponent exiles a card from their hand or a permanent they control"
        ) || rendered.contains(
            "+1: Draw a card. Each opponent exiles a card from their hand or a permanent they control"
        ),
        "expected compiled text to preserve draw plus hand-or-permanent exile clauses, got {rendered}"
    );
    assert!(
        rendered.contains(
            "−8: Each opponent who doesn't control a legendary creature or planeswalker loses the game"
        ),
        "expected compiled text to preserve -8 conditional losing-game clause, got {rendered}"
    );
    assert!(
        !crate::cards::generated_definition_has_unimplemented_content(&def),
        "Nicol Bolas should compile without unsupported runtime markers: {ability_debug}"
    );
}

#[test]
pub(super) fn chandra_nalaar_minus_x_compiled_text_preserves_x_damage_surface() {
    let def = parse_oracle_card_definition("Chandra Nalaar");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains("−X: Chandra Nalaar deals X damage to target creature"),
        "expected Chandra's -X ability to preserve X damage wording, got {rendered}"
    );
    assert!(
        !rendered.contains("loyalty counters removed this way to target creature"),
        "Chandra's -X ability should not expand X damage to counters-removed wording, got {rendered}"
    );
    assert!(
        rendered.contains(
            "−8: Chandra Nalaar deals 10 damage to target player or planeswalker and each creature that player or that planeswalker's controller controls"
        ),
        "expected Chandra's -8 ability to preserve player-or-planeswalker controller wording, got {rendered}"
    );
    assert!(
        !rendered.contains("that player or that object's controller controls"),
        "Chandra's -8 ability should not collapse planeswalker controller wording to object-controller wording, got {rendered}"
    );
}

pub(super) fn nicol_bolas_test_planeswalker_with_loyalty_and_nonloyalty_abilities() -> CardDefinition
{
    let mut loyalty_gain_life = crate::ability::Ability::activated(
        crate::cost::TotalCost::free(),
        vec![crate::effect::Effect::gain_life(3)],
    );
    if let AbilityKind::Activated(activated) = &mut loyalty_gain_life.kind {
        activated.is_loyalty_ability = true;
    }

    CardDefinitionBuilder::new(CardId::new(), "Bolas Test Planeswalker")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .with_ability(loyalty_gain_life)
        .with_ability(crate::ability::Ability::activated(
            crate::cost::TotalCost::free(),
            vec![crate::effect::Effect::draw(2)],
        ))
        .build()
}

#[test]
pub(super) fn nicol_bolas_dragon_god_copies_only_other_planeswalkers_loyalty_abilities() {
    let nicol = parse_oracle_card_definition("Nicol Bolas, Dragon-God");
    let other_planeswalker = nicol_bolas_test_planeswalker_with_loyalty_and_nonloyalty_abilities();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let nicol_id = game.create_object_from_definition(&nicol, alice, Zone::Battlefield);
    game.create_object_from_definition(&other_planeswalker, bob, Zone::Battlefield);

    let activated = game
        .current_abilities(nicol_id)
        .expect("Nicol Bolas should have current abilities")
        .into_iter()
        .filter_map(|ability| match ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .collect::<Vec<_>>();
    let loyalty_count = activated
        .iter()
        .filter(|ability| ability.is_loyalty_ability())
        .count();
    assert_eq!(
        loyalty_count, 4,
        "Nicol Bolas should have his three printed loyalty abilities plus one copied loyalty ability, got {activated:#?}"
    );
    assert!(
        activated.iter().any(|ability| ability
            .effects
            .flattened_default_effects()
            .into_iter()
            .any(|effect| effect
                .downcast_ref::<crate::effects::GainLifeEffect>()
                .is_some())),
        "Nicol Bolas should copy the other planeswalker's loyalty ability, got {activated:#?}"
    );
    assert!(
        !activated.iter().any(|ability| ability
            .effects
            .flattened_default_effects()
            .into_iter()
            .any(|effect| effect
                .downcast_ref::<crate::effects::DrawCardsEffect>()
                .is_some_and(|draw| draw.count == Value::Fixed(2)))),
        "Nicol Bolas must not copy non-loyalty activated abilities, got {activated:#?}"
    );
}

pub(super) fn nicol_bolas_plus_one_effects(def: &CardDefinition) -> Vec<Effect> {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.is_loyalty_ability() && {
                    let debug = format!("{:?}", activated.effects);
                    debug.contains("ChooseObjectsEffect")
                        && debug.contains("zone: Some(Hand)")
                        && debug.contains("additional_zones: [Battlefield]")
                        && debug.contains("MoveToZoneEffect")
                        && debug.contains("zone: Exile")
                } =>
            {
                Some(
                    activated
                        .effects
                        .flattened_default_effects()
                        .into_iter()
                        .cloned()
                        .collect(),
                )
            }
            _ => None,
        })
        .expect("Nicol Bolas should have a +1 draw and each-opponent exile loyalty ability")
}

pub(super) fn nicol_bolas_test_card(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(card_types)
        .build()
}

pub(super) fn nicol_bolas_has_object_named_in_zone(
    game: &crate::game_state::GameState,
    name: &str,
    zone: Zone,
) -> bool {
    game.objects_in_zone(zone)
        .into_iter()
        .any(|id| game.object(id).is_some_and(|object| object.name == name))
}

#[test]
pub(super) fn nicol_bolas_dragon_god_plus_one_each_opponent_exiles_own_hand_card_or_permanent() {
    struct ChooseExpectedOpponentObjects {
        bob_choice: ObjectId,
        charlie_choice: ObjectId,
        prompts: Vec<PlayerId>,
    }

    impl crate::decision::DecisionMaker for ChooseExpectedOpponentObjects {
        fn decide_objects(
            &mut self,
            game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            let choice = match ctx.player.index() {
                1 => self.bob_choice,
                2 => self.charlie_choice,
                _ => panic!("Nicol Bolas +1 should prompt only opponents, got {ctx:?}"),
            };

            assert_eq!(ctx.min, 1, "each opponent should choose exactly one object");
            assert_eq!(
                ctx.max,
                Some(1),
                "each opponent should choose exactly one object"
            );
            for candidate in &ctx.candidates {
                assert!(candidate.legal, "all surfaced candidates should be legal");
                let object = game
                    .object(candidate.id)
                    .expect("choice candidate should exist");
                assert!(
                    (object.zone == Zone::Hand && object.owner == ctx.player)
                        || (object.zone == Zone::Battlefield
                            && game.controller_of(object) == ctx.player),
                    "Nicol Bolas +1 candidates must be from the chooser's hand or permanents they control, got {object:#?} for {ctx:?}"
                );
            }
            assert!(
                ctx.candidates
                    .iter()
                    .any(|candidate| candidate.id == choice),
                "expected chosen object to be among candidates for {ctx:?}"
            );
            self.prompts.push(ctx.player);
            vec![choice]
        }
    }

    let nicol = parse_oracle_card_definition("Nicol Bolas, Dragon-God");
    let plus_one_effects = nicol_bolas_plus_one_effects(&nicol);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let mut game = crate::game_state::GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let source = game.create_object_from_definition(&nicol, alice, Zone::Battlefield);
    let alice_hand = game.create_object_from_definition(
        &nicol_bolas_test_card("Alice Hand Card", vec![CardType::Sorcery]),
        alice,
        Zone::Hand,
    );
    let alice_permanent = game.create_object_from_definition(
        &nicol_bolas_test_card("Alice Permanent", vec![CardType::Artifact]),
        alice,
        Zone::Battlefield,
    );
    game.create_object_from_definition(
        &nicol_bolas_test_card("Alice Draw Card", vec![CardType::Instant]),
        alice,
        Zone::Library,
    );
    let bob_hand = game.create_object_from_definition(
        &nicol_bolas_test_card("Bob Hand Card", vec![CardType::Sorcery]),
        bob,
        Zone::Hand,
    );
    let bob_permanent = game.create_object_from_definition(
        &nicol_bolas_test_card("Bob Permanent", vec![CardType::Artifact]),
        bob,
        Zone::Battlefield,
    );
    let charlie_hand = game.create_object_from_definition(
        &nicol_bolas_test_card("Charlie Hand Card", vec![CardType::Sorcery]),
        charlie,
        Zone::Hand,
    );
    let charlie_permanent = game.create_object_from_definition(
        &nicol_bolas_test_card("Charlie Permanent", vec![CardType::Artifact]),
        charlie,
        Zone::Battlefield,
    );

    let mut dm = ChooseExpectedOpponentObjects {
        bob_choice: bob_hand,
        charlie_choice: charlie_permanent,
        prompts: Vec::new(),
    };
    {
        let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
        for effect in &plus_one_effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx)
                .expect("Nicol Bolas +1 should resolve");
        }
    }

    assert_eq!(dm.prompts, vec![bob, charlie]);
    assert!(
        nicol_bolas_has_object_named_in_zone(&game, "Bob Hand Card", Zone::Exile),
        "Bob's chosen hand card should move to exile"
    );
    assert!(
        nicol_bolas_has_object_named_in_zone(&game, "Charlie Permanent", Zone::Exile),
        "Charlie's chosen permanent should move to exile"
    );
    assert_eq!(
        game.object(bob_permanent).expect("bob permanent").zone,
        Zone::Battlefield,
        "Bob should exile exactly one object"
    );
    assert_eq!(
        game.object(charlie_hand).expect("charlie hand card").zone,
        Zone::Hand,
        "Charlie should exile exactly one object"
    );
    assert_eq!(
        game.object(alice_hand).expect("alice hand card").zone,
        Zone::Hand,
        "Nicol Bolas's controller should not choose or exile a card"
    );
    assert_eq!(
        game.object(alice_permanent).expect("alice permanent").zone,
        Zone::Battlefield,
        "Nicol Bolas's controller should not choose or exile a permanent"
    );
}

pub(super) fn nicol_bolas_minus_eight_effects(def: &CardDefinition) -> Vec<Effect> {
    def.abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated)
                if activated.is_loyalty_ability()
                    && activated
                        .effects
                        .flattened_default_effects()
                        .into_iter()
                        .any(|effect| format!("{effect:#?}").contains("LoseTheGameEffect")) =>
            {
                Some(
                    activated
                        .effects
                        .flattened_default_effects()
                        .into_iter()
                        .cloned()
                        .collect(),
                )
            }
            _ => None,
        })
        .expect("Nicol Bolas should have a -8 lose-the-game loyalty ability")
}

#[test]
pub(super) fn nicol_bolas_dragon_god_minus_eight_respects_legendary_creature_or_planeswalker_branch()
 {
    let nicol = parse_oracle_card_definition("Nicol Bolas, Dragon-God");
    let minus_eight_effects = nicol_bolas_minus_eight_effects(&nicol);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let mut unprotected_game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let unprotected_source =
        unprotected_game.create_object_from_definition(&nicol, alice, Zone::Battlefield);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(unprotected_source, alice, &mut dm);
    for effect in &minus_eight_effects {
        crate::effects::execute_effect(&mut unprotected_game, effect, &mut ctx)
            .expect("Nicol Bolas -8 should resolve");
    }
    assert!(
        unprotected_game.player(bob).expect("bob exists").has_lost,
        "opponent without a legendary creature or planeswalker should lose the game"
    );
    assert!(
        !unprotected_game
            .player(alice)
            .expect("alice exists")
            .has_lost,
        "Nicol Bolas -8 should affect opponents only"
    );

    let legendary_creature = CardDefinitionBuilder::new(CardId::new(), "Bolas Test Legend")
        .supertypes(vec![Supertype::Legendary])
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let mut protected_game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let protected_source =
        protected_game.create_object_from_definition(&nicol, alice, Zone::Battlefield);
    protected_game.create_object_from_definition(&legendary_creature, bob, Zone::Battlefield);
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(protected_source, alice, &mut dm);
    for effect in &minus_eight_effects {
        crate::effects::execute_effect(&mut protected_game, effect, &mut ctx)
            .expect("Nicol Bolas -8 should resolve");
    }
    assert!(
        !protected_game.player(bob).expect("bob exists").has_lost,
        "opponent with a legendary creature should not lose the game"
    );

    let legendary_planeswalker =
        CardDefinitionBuilder::new(CardId::new(), "Bolas Test Legendwalker")
            .supertypes(vec![Supertype::Legendary])
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build();
    let mut planeswalker_protected_game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let planeswalker_protected_source =
        planeswalker_protected_game.create_object_from_definition(&nicol, alice, Zone::Battlefield);
    planeswalker_protected_game.create_object_from_definition(
        &legendary_planeswalker,
        bob,
        Zone::Battlefield,
    );
    let mut dm = crate::decision::SelectFirstDecisionMaker;
    let mut ctx =
        crate::effects::ExecutionContext::new(planeswalker_protected_source, alice, &mut dm);
    for effect in &minus_eight_effects {
        crate::effects::execute_effect(&mut planeswalker_protected_game, effect, &mut ctx)
            .expect("Nicol Bolas -8 should resolve");
    }
    assert!(
        !planeswalker_protected_game
            .player(bob)
            .expect("bob exists")
            .has_lost,
        "opponent with a legendary planeswalker should not lose the game"
    );
}

#[test]
pub(super) fn ichormoon_gauntlet_trigger_adds_selected_existing_counter_kind() {
    struct ChooseFirstCounterKind;
    impl crate::decision::DecisionMaker for ChooseFirstCounterKind {
        fn decide_options(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if ctx.description == "Choose a counter kind" {
                vec![0]
            } else {
                vec![0]
            }
        }
    }

    let def = parse_oracle_card_definition("Ichormoon Gauntlet");
    let effect = ichormoon_gauntlet_chosen_counter_effect(&def);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Countered Permanent")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );
    game.add_counters(target, CounterType::Charge, 2)
        .expect("target should accept charge counters");
    game.add_counters(target, CounterType::PlusOnePlusOne, 1)
        .expect("target should accept +1/+1 counters");

    let mut dm = ChooseFirstCounterKind;
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    crate::effects::execute_effect(&mut game, effect, &mut ctx)
        .expect("chosen-kind counter effect should resolve");

    assert_eq!(
        game.counter_count(target, CounterType::Charge),
        3,
        "chosen charge counter kind should receive one additional counter"
    );
    assert_eq!(
        game.counter_count(target, CounterType::PlusOnePlusOne),
        1,
        "unchosen counter kinds should not change"
    );
    assert_eq!(
        game.counter_count(target, CounterType::Named("additional")),
        0,
        "the effect must not create a literal named additional counter"
    );
}

#[test]
pub(super) fn ichormoon_gauntlet_trigger_does_nothing_when_target_has_no_counters() {
    let def = parse_oracle_card_definition("Ichormoon Gauntlet");
    let effect = ichormoon_gauntlet_chosen_counter_effect(&def);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    let target = game.create_object_from_card(
        &CardBuilder::new(CardId::new(), "Uncountered Permanent")
            .card_types(vec![CardType::Artifact])
            .build(),
        bob,
        Zone::Battlefield,
    );

    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
    crate::effects::execute_effect(&mut game, effect, &mut ctx)
        .expect("chosen-kind counter effect should safely resolve with no counters");

    assert_eq!(
        game.object(target)
            .expect("target should still exist")
            .counters
            .values()
            .sum::<u32>(),
        0,
        "a target with no counters should not receive any counter"
    );
}

#[test]
pub(super) fn reprocess_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Reprocess");

    let def = parse_oracle_card_definition("Reprocess");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(def.name(), "Reprocess");
    assert_eq!(def.card.card_types, vec![CardType::Sorcery]);
    let effects = def
        .spell_effect
        .as_ref()
        .expect("Reprocess should have spell effects")
        .flattened_default_effects();
    let [choose_effect, sacrifice_effect, draw_effect] = effects else {
        panic!("Reprocess should lower to choose, sacrifice, and draw effects, got {effects:#?}");
    };
    let choose = choose_effect
        .downcast_ref::<crate::effects::ChooseObjectsEffect>()
        .expect("Reprocess should choose permanents to sacrifice");
    let sacrifice_with_id = sacrifice_effect
        .downcast_ref::<crate::effects::WithIdEffect>()
        .expect("Reprocess sacrifice should be effect-metric addressable");
    let sacrifice = sacrifice_with_id
        .effect
        .downcast_ref::<crate::effects::zones::SacrificePlayerEffect>()
        .expect("Reprocess should sacrifice the chosen permanents");
    let draw = draw_effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .expect("Reprocess should draw from the sacrificed count");

    assert_eq!(choose.filter.controller, Some(PlayerFilter::You));
    assert_eq!(
        choose.filter.card_types,
        vec![CardType::Artifact, CardType::Creature, CardType::Land]
    );
    assert_eq!(sacrifice.player, PlayerFilter::You);
    let draw_count = draw.count.unhinted();
    assert!(
        matches!(
            draw_count,
            Value::EffectMetric {
                effect_id,
                source:
                    crate::effect::EffectMetricSource::AffectedObjects
                    | crate::effect::EffectMetricSource::Outcome,
                metric: crate::effect::EffectMetric::Count,
            } if *effect_id == sacrifice_with_id.id
        ) || matches!(
            draw_count,
            Value::PriorEffectMetric { effect_id, query }
                if *effect_id == sacrifice_with_id.id
                    && query.source == crate::effect::EffectMetricSource::AffectedObjects
                    && query.metric == crate::effect::EffectMetric::Count
        ),
        "Reprocess draw count should reference the sacrificed-object metric, got {:?}",
        draw.count
    );
    assert!(
        rendered.contains(
            "Sacrifice any number of artifacts, creatures, and/or lands. Draw a card for each permanent sacrificed this way."
        ),
        "Reprocess should render its sacrificed-this-way draw clause, got {rendered}"
    );
}

#[test]
pub(super) fn necromancers_covenant_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Necromancer's Covenant");

    let def = parse_oracle_card_definition("Necromancer's Covenant");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert_eq!(def.name(), "Necromancer's Covenant");
    assert_eq!(def.card.card_types, vec![CardType::Enchantment]);
    assert!(
        rendered.contains(
            "When this enchantment enters, exile all creature cards from target player's graveyard, then create a 2/2 black Zombie creature token for each card exiled this way."
        ),
        "Necromancer's Covenant should render its exiled-this-way token count, got {rendered}"
    );
    assert!(
        !rendered.contains("target player creates"),
        "bare create after target-player graveyard exile should be controlled by you, got {rendered}"
    );
    assert!(
        rendered.contains("Zombies you control have lifelink"),
        "Necromancer's Covenant should render the Zombie lifelink grant, got {rendered}"
    );
    assert!(
        ability_debug.contains("ExileEffect")
            && ability_debug.contains("CreateTokenEffect")
            && ability_debug.contains("EffectMetric")
            && ability_debug.contains("AffectedObjects")
            && ability_debug.contains("GrantObjectAbilityForFilter")
            && ability_debug.contains("Lifelink"),
        "Necromancer's Covenant should structurally exile, count affected cards, create Zombies, and grant lifelink, got {ability_debug}"
    );
}

#[test]
pub(super) fn explicit_target_player_create_after_exile_keeps_target_controller() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(405_318), "Explicit Target Create")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile all creature cards from target player's graveyard, then target player creates a 2/2 black Zombie creature token for each card exiled this way.",
        )
        .expect("explicit target-player create should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("target player creates"),
        "explicit target-player token creation should not be rewritten to you, got {rendered}"
    );
}

#[test]
pub(super) fn filtered_exiled_this_way_count_is_not_lowered_to_unfiltered_metric() {
    let result = CardDefinitionBuilder::new(CardId::from_raw(405_319), "Filtered Count")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Exile all cards from target player's graveyard, then create a 2/2 black Zombie creature token for each creature card exiled this way.",
        );

    assert!(
        result.is_err(),
        "filtered exiled-this-way counts need filtered metric support, not an unfiltered affected-object count"
    );
}

#[test]
pub(super) fn boss_s_chauffeur_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Boss's Chauffeur");

    let def = parse_oracle_card_definition("Boss's Chauffeur");
    let rendered = unprocessed_compiled_lines(&def).join("\n");

    assert!(
        rendered.contains(
            "This creature enters with a number of +1/+1 counters on it equal to one plus the number of other creatures you control."
        ),
        "Boss's Chauffeur should render the equal-to ETB counter count, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Alliance — Whenever another creature you control enters, put a +1/+1 counter on this creature."
        ),
        "Boss's Chauffeur should render its Alliance trigger, got {rendered}"
    );
    assert!(
        rendered.contains(
            "When this creature dies, create a 1/1 green and white Citizen creature token for each +1/+1 counter on it."
        ),
        "Boss's Chauffeur should render token creation for each +1/+1 counter on it, got {rendered}"
    );
}

#[test]
pub(super) fn nymris_oonas_trickster_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Nymris, Oona's Trickster");

    let def = parse_oracle_card_definition("Nymris, Oona's Trickster");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let ability_debug = format!("{:#?}", def.abilities);

    assert!(
        rendered.lines().any(|line| line == "Flash")
            && rendered.lines().any(|line| line == "Flying"),
        "Nymris should preserve its separate keyword abilities, got {rendered}"
    );
    assert!(
        rendered.contains(
            "Whenever you cast your first spell during each opponent's turn, look at the top two cards of your library. Put one of them into your hand and the other into your graveyard."
        ),
        "Nymris should render its first-spell loot trigger, got {rendered}"
    );
    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Nymris should have a triggered ability");
    assert_eq!(
        triggered.trigger.display(),
        "Whenever you cast your first spell during each opponent's turn"
    );
    let effects_debug = format!("{:#?}", triggered.effects.flattened_default_effects());
    assert!(
        ability_debug.contains("SpellCastTrigger")
            && ability_debug.contains("during_turn: Some(")
            && ability_debug.contains("Opponent"),
        "expected Nymris to lower to a spell-cast trigger during opponents' turns, got {ability_debug}"
    );
    assert!(
        effects_debug.contains("LookAtTopCardsEffect")
            && effects_debug.contains("ChooseObjectsEffect")
            && effects_debug.contains("MoveToZoneEffect")
            && effects_debug.contains("Hand,")
            && effects_debug.contains("Graveyard,"),
        "expected Nymris to look at two cards, choose one for hand, and move the other to graveyard, got {effects_debug}"
    );
}

#[test]
pub(super) fn archon_of_coronation_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Archon of Coronation");
    let ability_debug = format!("{:#?}", def.abilities);
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        def.abilities
            .iter()
            .any(|ability| matches!(ability.kind, AbilityKind::Triggered(_))),
        "Archon of Coronation should parse its enters trigger strictly"
    );
    assert!(
        ability_debug.contains("BecomeMonarchEffect")
            && ability_debug.contains("DamageCauseLifeLoss")
            && ability_debug.contains("PlayerIsMonarch"),
        "expected monarch trigger and conditional damage-life-loss restriction, got {ability_debug}"
    );
    assert!(
        rendered_lower.contains("you become the monarch")
            && rendered_lower.contains("damage doesn't cause you to lose life")
            && rendered_lower.contains("as long as you're the monarch"),
        "expected Archon compiled text to preserve monarch and damage-life-loss clauses, got {rendered}"
    );
}

#[test]
pub(super) fn plague_of_vermin_strict_parser_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Plague of Vermin");
    let def = parse_oracle_card_definition("Plague of Vermin");
    let effect_debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        effect_debug.contains("RepeatProcessEffect")
            && effect_debug.contains("PayAnyLifeEffect")
            && effect_debug.contains("ForPlayersEffect")
            && effect_debug.contains("CreateTokenEffect")
            && effect_debug.contains("IteratedPlayerCount"),
        "Plague of Vermin should lower to repeat per-player life payments and per-player token counts, got {effect_debug}"
    );
    assert!(
        rendered.contains("Starting with you, each player may pay any amount of life")
            && rendered.contains("Repeat this process until no one pays life")
            && rendered.contains(
                "Each player creates a 1/1 black Rat creature token for each 1 life they paid this way"
            ),
        "expected repeated life-payment token text to be preserved, got {rendered}"
    );
}

#[test]
pub(super) fn plague_of_vermin_runtime_uses_each_players_paid_life_for_rat_tokens() {
    struct PlaguePayments {
        payments: Vec<u32>,
        next: usize,
    }

    impl crate::decision::DecisionMaker for PlaguePayments {
        fn decide_number(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            let payment = self.payments.get(self.next).copied().unwrap_or(ctx.min);
            self.next += 1;
            payment.clamp(ctx.min, ctx.max)
        }
    }

    fn rat_tokens_controlled_by(
        game: &crate::game_state::GameState,
        controller: PlayerId,
    ) -> usize {
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter(|id| {
                let Some(object) = game.object(*id) else {
                    return false;
                };
                object.kind == crate::object::ObjectKind::Token
                    && object.name == "Rat"
                    && object.card_types.contains(&CardType::Creature)
                    && object.subtypes.contains(&Subtype::Rat)
                    && object.color_override == Some(crate::color::ColorSet::BLACK)
                    && matches!(object.base_power, Some(crate::card::PtValue::Fixed(1)))
                    && matches!(object.base_toughness, Some(crate::card::PtValue::Fixed(1)))
                    && game.controller_of(object) == controller
            })
            .count()
    }

    let def = parse_oracle_card_definition("Plague of Vermin");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = PlaguePayments {
        payments: vec![2, 1, 0, 0],
        next: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let effects = &def
        .spell_effect
        .as_ref()
        .expect("Plague of Vermin should have spell effects")
        .segments[0]
        .default_effects;

    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Plague of Vermin effect should resolve");
    }

    assert_eq!(
        game.player(alice).expect("alice").life,
        18,
        "Alice should pay exactly 2 life across the repeated process"
    );
    assert_eq!(
        game.player(bob).expect("bob").life,
        19,
        "Bob should pay exactly 1 life across the repeated process"
    );
    assert_eq!(rat_tokens_controlled_by(&game, alice), 2);
    assert_eq!(rat_tokens_controlled_by(&game, bob), 1);
    assert_eq!(
        dm.next, 4,
        "the repeated process should ask both players again and stop when no one pays"
    );
}

#[test]
pub(super) fn plague_of_vermin_runtime_sums_life_paid_across_multiple_rounds() {
    struct PlaguePayments {
        payments: Vec<u32>,
        next: usize,
    }

    impl crate::decision::DecisionMaker for PlaguePayments {
        fn decide_number(
            &mut self,
            _game: &crate::game_state::GameState,
            ctx: &crate::decisions::context::NumberContext,
        ) -> u32 {
            let payment = self.payments.get(self.next).copied().unwrap_or(ctx.min);
            self.next += 1;
            payment.clamp(ctx.min, ctx.max)
        }
    }

    fn rat_tokens_controlled_by(
        game: &crate::game_state::GameState,
        controller: PlayerId,
    ) -> usize {
        game.objects_in_zone(Zone::Battlefield)
            .into_iter()
            .filter(|id| {
                let Some(object) = game.object(*id) else {
                    return false;
                };
                object.kind == crate::object::ObjectKind::Token
                    && object.name == "Rat"
                    && game.controller_of(object) == controller
            })
            .count()
    }

    let def = parse_oracle_card_definition("Plague of Vermin");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);
    let mut dm = PlaguePayments {
        payments: vec![2, 1, 3, 0, 0, 0],
        next: 0,
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm);
    let effects = &def
        .spell_effect
        .as_ref()
        .expect("Plague of Vermin should have spell effects")
        .segments[0]
        .default_effects;

    for effect in effects {
        crate::effects::execute_effect(&mut game, effect, &mut ctx)
            .expect("Plague of Vermin effect should resolve");
    }

    assert_eq!(game.player(alice).expect("alice").life, 15);
    assert_eq!(game.player(bob).expect("bob").life, 19);
    assert_eq!(rat_tokens_controlled_by(&game, alice), 5);
    assert_eq!(rat_tokens_controlled_by(&game, bob), 1);
    assert_eq!(dm.next, 6);
}
