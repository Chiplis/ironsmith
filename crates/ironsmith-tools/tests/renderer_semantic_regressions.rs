//! Regressions from the 2026-10-23 renderer–semantics audit
//! (reports/renderer-semantic-audit-2026-10-23/report.md).
//!
//! Two families:
//! - gameplay boundaries where correct-looking compiled text hid a wrong
//!   executable model (aggregate vs. object count, set constraints, OR
//!   branches, resolution references, attachment hosts);
//! - audit-renderer invariants: retained prose never reaches the semantic
//!   audit rendering, and executable mutations always change it.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::builders::CardDefinitionBuilder;
use ironsmith::game_state::GameState;
use ironsmith::ids::{CardId, PlayerId};
use ironsmith::static_abilities::{StaticAbility, StaticAbilityId};
use ironsmith::types::CardType as T;
use ironsmith::zone::Zone;
use ironsmith::{CardDefinition, ConditionExpr};
use ironsmith_core::StaticAbilityPayload as P;
use ironsmith_core::semantic_audit::UNVERIFIED_MARKER_PREFIX;

fn on_big_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

fn compile(name: &str) -> CardDefinition {
    // The audit family tests compile many named cards. Scan the catalog once
    // for that shared set instead of reparsing it for every test case.
    static PAYLOADS: std::sync::OnceLock<
        std::collections::BTreeMap<String, Vec<ironsmith_tools::CardPayload>>,
    > = std::sync::OnceLock::new();
    let payloads = PAYLOADS.get_or_init(|| {
        let names = AUDITED
            .iter()
            .chain(FAMILY_REPRESENTATIVES)
            .chain(
                [
                    "Elephant Grass",
                    "Earth",
                    "Chaos Moon",
                    "Surveillance Phantasm",
                    "Shatterskull Charger",
                    "Rhet-Crop Spearmaster",
                    "Hammerhead Shark",
                    "Embermaw Hellion",
                    "Etching of Kumano",
                    "Titania's Song",
                    "Cogwork Tracker",
                    "Plague Nurse",
                    "Lore Seeker",
                    "Elemental Time Flamingo",
                ]
                .iter(),
            )
            .map(|name| (*name).to_owned())
            .collect::<Vec<_>>();
        ironsmith_tools::load_card_payloads_by_names("../../cards.json", &names).unwrap()
    });
    if let Some(payload) = payloads
        .get(&ironsmith_tools::normalize_lookup_name(name))
        .and_then(|p| p.first())
    {
        return ironsmith_tools::compile_runtime_definition_from_payload(payload).unwrap();
    }
    let payloads = ironsmith_tools::load_card_payloads_by_name("../../cards.json", name).unwrap();
    ironsmith_tools::compile_runtime_definition_from_payload(payloads.first().unwrap()).unwrap()
}

fn witness(name: &str, types: Vec<T>) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(types)
        .build()
}

fn creature(name: &str, power: i32) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![T::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(power, 3))
        .build()
}

fn static_condition(def: &CardDefinition) -> ConditionExpr {
    def.abilities
        .iter()
        .find_map(|ability| {
            let AbilityKind::Static(s) = &ability.kind else {
                return None;
            };
            match &s.compiled_model()?.payload {
                P::Anthem(anthem) => anthem.condition.clone(),
                P::Conditional { condition, .. } => Some(condition.clone()),
                _ => None,
            }
        })
        .expect("static gate")
}

fn external(game: &GameState, condition: &ConditionExpr, source: ironsmith::ObjectId) -> bool {
    let context = ironsmith::condition_eval::ExternalEvaluationContext {
        source,
        controller: PlayerId::from_index(0),
        filter_source: Some(source),
        ..Default::default()
    };
    ironsmith::condition_eval::evaluate_condition_external(game, condition, &context)
}

#[test]
fn keen_eyed_curator_counts_card_types_not_cards() {
    on_big_stack(|| {
        let def = compile("Keen-Eyed Curator");
        let cases: [(&[&[T]], bool); 3] = [
            (
                &[&[T::Creature], &[T::Creature], &[T::Artifact], &[T::Land]],
                false,
            ),
            (
                &[
                    &[T::Creature],
                    &[T::Creature],
                    &[T::Creature],
                    &[T::Creature],
                ],
                false,
            ),
            (
                &[&[T::Artifact, T::Creature], &[T::Enchantment], &[T::Land]],
                true,
            ),
        ];
        for (exiled, buffed) in cases {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let p = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&def, p, Zone::Battlefield);
            for (i, types) in exiled.iter().enumerate() {
                let id = game.create_object_from_definition(
                    &witness(&format!("Exiled {i}"), types.to_vec()),
                    p,
                    Zone::Exile,
                );
                game.add_exiled_with_source_link(source, id);
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                game.calculated_power(source),
                Some(if buffed { 7 } else { 3 }),
                "{exiled:?}"
            );
            assert_eq!(
                game.object_has_static_ability_id(source, StaticAbilityId::Trample),
                buffed,
                "{exiled:?}"
            );
        }
    });
}

#[test]
fn augur_of_autumn_coven_requires_three_different_powers() {
    on_big_stack(|| {
        let def = compile("Augur of Autumn");
        let condition = static_condition(&def);
        for (others, expected) in [([2, 2], false), ([1, 3], true)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let p = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&def, p, Zone::Battlefield);
            for (i, power) in others.into_iter().enumerate() {
                game.create_object_from_definition(
                    &creature(&format!("Witness {i}"), power),
                    p,
                    Zone::Battlefield,
                );
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(external(&game, &condition, source), expected, "{others:?}");
        }
    });
}

#[test]
fn earth_rumble_wrestlers_either_branch_satisfies_the_gate() {
    on_big_stack(|| {
        let def = compile("Earth Rumble Wrestlers");
        let condition = static_condition(&def);
        for (types, expected) in [
            (vec![T::Land, T::Creature], true),
            (vec![T::Creature], false),
        ] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let p = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&def, p, Zone::Battlefield);
            let w = CardDefinitionBuilder::new(CardId::new(), "Witness")
                .card_types(types.clone())
                .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
                .build();
            game.create_object_from_definition(&w, p, Zone::Battlefield);
            game.turn_store.turn_history = Default::default();
            game.refresh_continuous_state().unwrap();
            assert_eq!(external(&game, &condition, source), expected, "{types:?}");
        }
    });
}

#[test]
fn brass_knuckles_counts_equipment_on_the_equipped_creature() {
    on_big_stack(|| {
        let def = compile("Brass Knuckles");
        for second in [false, true] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let p = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&def, p, Zone::Battlefield);
            let host =
                game.create_object_from_definition(&creature("Host", 2), p, Zone::Battlefield);
            assert!(game.attach_object_to_target(
                source,
                ironsmith::object::AttachmentTarget::Object(host)
            ));
            if second {
                let eq = CardDefinitionBuilder::new(CardId::new(), "Other Equipment")
                    .card_types(vec![T::Artifact])
                    .subtypes(vec![ironsmith::types::Subtype::Equipment])
                    .build();
                let eq = game.create_object_from_definition(&eq, p, Zone::Battlefield);
                assert!(game.attach_object_to_target(
                    eq,
                    ironsmith::object::AttachmentTarget::Object(host)
                ));
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                game.object_has_static_ability_id(host, StaticAbilityId::DoubleStrike),
                second
            );
        }
    });
}

fn resolution_conditions(def: &CardDefinition) -> Vec<ConditionExpr> {
    fn find(effect: &ironsmith::Effect, out: &mut Vec<ConditionExpr>) {
        if let Some(c) = effect.downcast_ref::<ironsmith::effects::ConditionalEffect>() {
            out.push(c.condition.clone());
        }
        effect.visit_child_effects(&mut |child| find(child, out));
    }
    let mut out = vec![];
    for ability in &def.abilities {
        let effects = match &ability.kind {
            AbilityKind::Activated(a) => a.effects.all_effects(),
            AbilityKind::Triggered(t) => {
                out.extend(t.intervening_if.clone());
                t.effects.all_effects()
            }
            _ => vec![],
        };
        for effect in effects {
            find(effect, &mut out);
        }
    }
    if let Some(program) = &def.spell_effect {
        for effect in program.all_effects() {
            find(effect, &mut out);
        }
    }
    out
}

/// Evaluate `condition` with `__it__` bound to an object named `it_name` in
/// `it_zone`, against a reference card named "Matching Card" in `ref_zone`.
fn same_name_gate(
    def: &CardDefinition,
    condition: &ConditionExpr,
    ty: T,
    ref_zone: Zone,
    it_zone: Zone,
    it_name: &str,
    extra_tags: &[&str],
) -> bool {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let p = PlayerId::from_index(0);
    let source = game.create_object_from_definition(def, p, Zone::Battlefield);
    let reference =
        game.create_object_from_definition(&witness("Matching Card", vec![ty]), p, ref_zone);
    if ref_zone == Zone::Exile {
        game.add_exiled_with_source_link(source, reference);
    }
    let tested = game.create_object_from_definition(&witness(it_name, vec![ty]), p, it_zone);
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    let mut ctx = ironsmith::effects::EffectContext::new(source, p, &mut dm);
    ctx.tag_object(
        ironsmith::TagKey::from("__it__"),
        ironsmith::snapshot::ObjectSnapshot::from_object(game.object(tested).unwrap(), &game),
    );
    for tag in extra_tags {
        ctx.tag_object(
            ironsmith::TagKey::from(*tag),
            ironsmith::snapshot::ObjectSnapshot::from_object(
                game.object(reference).unwrap(),
                &game,
            ),
        );
    }
    ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap()
}

#[test]
fn resolution_same_name_counts_keep_their_bound_reference() {
    on_big_stack(|| {
        for (name, ty, ref_zone, it_zone, negated) in [
            (
                "Candles of Leng",
                T::Sorcery,
                Zone::Graveyard,
                Zone::Stack,
                false,
            ),
            (
                "Retraced Image",
                T::Creature,
                Zone::Battlefield,
                Zone::Hand,
                false,
            ),
            (
                "Guardian Project",
                T::Creature,
                Zone::Graveyard,
                Zone::Battlefield,
                true,
            ),
        ] {
            let def = compile(name);
            let condition = resolution_conditions(&def)
                .into_iter()
                .find(|c| format!("{c:?}").contains("SameNameAsTagged"))
                .expect("same-name gate");
            for same in [true, false] {
                let it_name = if same { "Matching Card" } else { "Other Card" };
                assert_eq!(
                    same_name_gate(&def, &condition, ty, ref_zone, it_zone, it_name, &[]),
                    same != negated,
                    "{name} same={same}"
                );
            }
        }
    });
}

#[test]
fn jesters_scepter_compares_target_spell_with_the_paid_card() {
    on_big_stack(|| {
        let def = compile("Jester's Scepter");
        let condition = resolution_conditions(&def)
            .into_iter()
            .next()
            .expect("gate");
        let debug = format!("{condition:?}");
        assert!(
            debug.contains("zone_cost_0"),
            "paid card is its own operand: {debug}"
        );
        for same in [true, false] {
            let it_name = if same { "Matching Card" } else { "Other Card" };
            assert_eq!(
                same_name_gate(
                    &def,
                    &condition,
                    T::Instant,
                    Zone::Graveyard,
                    Zone::Stack,
                    it_name,
                    &["zone_cost_0"],
                ),
                same
            );
        }
    });
}

fn audit_lines(def: &CardDefinition) -> Vec<String> {
    ironsmith::compiled_text::semantic_audit_lines(def)
}

#[path = "support/semantic_audit_probes.rs"]
mod semantic_audit_probes;
use semantic_audit_probes::{POISON, map_static_models, poison_definition};

const AUDITED: [&str; 8] = [
    "Keen-Eyed Curator",
    "Augur of Autumn",
    "Castle Ardenvale",
    "Suppression Field",
    "Bruvac the Grandiloquent",
    "Divine Presence",
    "Kenessos, Priest of Thassa",
    "Brass Knuckles",
];

/// One face per family that still renders an audit marker
/// (reports/renderer-semantic-audit-2026-10-23/audit-marker-families.json).
const FAMILY_REPRESENTATIVES: &[&str] = &[
    "Activated Sleeper",
    "Altar of the Pantheon",
    "Adaptive Automaton",
    "Abby, Merciless Soldier",
    "Abduction",
    "Ancient Greenwarden",
    "Alms Collector",
    "Archangel of Strife",
    "Agatha's Soul Cauldron",
    "Absorbing Man and Titania",
    "Benevolent Hydra",
    "Bitter Feud",
    "Bilbo, Fellow Conspirator",
    "Abundance",
    "Bender's Waterskin",
    "Case File Auditor",
    "Black Ward",
    "Breathstealer's Crypt",
    "Aether Refinery",
    "Arcane Adaptation",
    "Guardian Seraph",
    "Benevolent Unicorn",
    "Adrix and Nev, Twincasters",
    "Arixmethes, Slumbering Isle",
    "Eligeth, Crossroads Augur",
    "Alhammarret, High Arbiter",
    "Angel of Vitality",
    "Damping Field",
    "Alhammarret's Archive",
    "Adventurers' Guildhouse",
    "Alloy Golem",
    "Contamination",
    "Cairn Wanderer",
    "Convincing Mirage",
    "Doorkeeper Thrull",
    "Liquid Fire",
    "Absolute Virtue",
    "Aquamorph Entity",
    "Monitor Monitor",
    "Bard, King of Dale",
    "Chatterfang, Squirrel General",
    "Mana Reflection",
    "Exquisite Archangel",
    "Eye of Nidhogg",
    "Arsenal Thresher",
    "Dungeon Delver",
    "Shimmer",
    "Containment Priest",
    "Me, the Immortal",
    "Ancient Adamantoise",
    "Dinosaur Headdress",
    "Idris, Soul of the TARDIS",
    "Phantasmal Terrain",
    "Ironscale Hydra",
    "Chaos Lord",
    "Phytohydra",
    "Academy Manufactor",
    "Harsh Judgment",
    "Heightened Awareness",
    "Sigarda's Splendor",
    "Psychic Paper",
    "Master of the Hunt",
    "Sages of the Anima",
    "Notion Thief",
    "Sanctuary Blade",
    "Melira, the Living Cure",
    "Ojer Axonil, Deepest Might",
    "Ojer Taq, Deepest Foundation",
    "Mind Grind",
    "Igneous Elemental",
    "Soul-Scar Mage",
    "Illusionary Terrain",
    "Rasputin Dreamweaver",
    "Painter's Servant",
    "Realmwright",
    "Multiversal Passage",
    "Aven Mindcensor",
    "Storage Matrix",
    "Clandestine Chameleon",
    "Land Grant",
    "Vigor",
    "Gisela, Blade of Goldnight",
    "Volrath's Shapeshifter",
    "The Book of Vile Darkness",
    "Damping Sphere",
    "Wolverine, Fierce Fighter",
];

#[test]
fn semantic_audit_rendering_ignores_poisoned_presentation_text() {
    on_big_stack(|| {
        let mut leaks = vec![];
        for name in AUDITED.iter().chain(FAMILY_REPRESENTATIVES) {
            let def = compile(name);
            let before = audit_lines(&def);
            let after = audit_lines(&poison_definition(&def));
            if after.join("\n").contains(POISON) || after != before {
                leaks.push(format!("{name}:\n  {before:?}\n  {after:?}"));
            }
        }
        assert!(
            leaks.is_empty(),
            "audit rendering depends on presentation text:\n{}",
            leaks.join("\n")
        );
    });
}

#[test]
fn semantic_audit_rendering_reflects_executable_mutations() {
    use ironsmith_core::AmountModifierSpec;
    on_big_stack(|| {
        fn gate(c: &mut ConditionExpr) -> bool {
            match c {
                ConditionExpr::CountComparison { comparison, .. } => {
                    *comparison = ironsmith_core::Comparison::GreaterThanOrEqual(9);
                    true
                }
                ConditionExpr::ValueComparison { right, .. } => {
                    *right = ironsmith_core::Value::Fixed(9);
                    true
                }
                _ => false,
            }
        }
        for name in &AUDITED[..7] {
            let def = compile(name);
            let before = audit_lines(&def);
            let (mutated, any) = map_static_models(&def, |model| match &mut model.payload {
                P::Anthem(anthem) => anthem.condition.as_mut().is_some_and(gate),
                P::GrantObjectAbilityForFilter(grant) => grant.condition.as_mut().is_some_and(gate),
                P::EntersTappedUnlessCondition { condition, .. } => gate(condition),
                P::EventAmountReplacement { modifier, .. } => {
                    *modifier = AmountModifierSpec::SetTo(9);
                    true
                }
                P::ActivatedAbilityCostIncrease { increase, .. } => {
                    *increase =
                        ironsmith_core::TotalCost::mana(ironsmith::mana::ManaCost::from_symbols(
                            vec![ironsmith::mana::ManaSymbol::Generic(9)],
                        ));
                    true
                }
                P::Grants(grant) if grant.filtered_zone_surface.is_some() => {
                    grant.zone = Zone::Graveyard;
                    true
                }
                _ => false,
            });
            assert!(any, "{name}: no executable field was mutated");
            assert_ne!(
                audit_lines(&mutated),
                before,
                "{name}: audit hid a semantic change"
            );
        }
    });
}

#[test]
fn semantic_audit_markers_never_leak_into_ordinary_rendering() {
    on_big_stack(|| {
        for name in AUDITED {
            let lines = ironsmith::compiled_text::compiled_text_lines(&compile(name));
            assert!(
                !lines.join("\n").contains(UNVERIFIED_MARKER_PREFIX),
                "{name}: {lines:?}"
            );
        }
    });
}

mod jesters_scepter_lifecycle {
    use super::*;
    use ironsmith::decision::{
        GameProgress, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
    };
    use ironsmith::game_loop::{
        PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
        apply_priority_response_with_dm,
    };
    use ironsmith::triggers::TriggerQueue;

    fn drive(
        game: &mut GameState,
        queue: &mut TriggerQueue,
        action: LegalAction,
        done: impl Fn(&GameState, &PriorityLoopState) -> bool,
    ) {
        let mut dm = SelectFirstDecisionMaker;
        let mut state = PriorityLoopState::new(game.players_in_game());
        let mut progress = apply_priority_response_with_dm(
            game,
            queue,
            &mut state,
            &PriorityResponse::PriorityAction(action),
            &mut dm,
        )
        .unwrap();
        for _ in 0..32 {
            if done(game, &state) {
                return;
            }
            let GameProgress::NeedsDecisionCtx(ctx) = progress else {
                panic!("stalled: {progress:?}");
            };
            progress =
                apply_decision_context_with_dm(game, queue, &mut state, &ctx, &mut dm).unwrap();
        }
        panic!("decision bound exceeded");
    }

    fn resolve_top(game: &mut GameState, queue: &mut TriggerQueue) {
        let mut dm = SelectFirstDecisionMaker;
        let mut state = PriorityLoopState::new(game.players_in_game());
        state.reset_for_new_priority_window(game);
        for _ in 0..game.players_in_game() {
            apply_priority_response_with_dm(
                game,
                queue,
                &mut state,
                &PriorityResponse::PriorityAction(LegalAction::PassPriority),
                &mut dm,
            )
            .unwrap();
        }
    }

    /// Returns whether the targeted spell was countered.
    fn run(paid_name: &str) -> bool {
        let scepter = compile("Jester's Scepter");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let p = PlayerId::from_index(0);
        let mut queue = TriggerQueue::new();
        let source = game.create_object_from_definition(&scepter, p, Zone::Battlefield);
        let exiled = game.create_object_from_definition(
            &witness(paid_name, vec![T::Instant]),
            p,
            Zone::Exile,
        );
        game.add_exiled_with_source_link(source, exiled);
        for symbol in [
            ironsmith::mana::ManaSymbol::Blue,
            ironsmith::mana::ManaSymbol::Colorless,
        ] {
            game.player_mut(p).unwrap().mana_pool.add(symbol, 10);
        }
        game.turn.priority_player = Some(p);
        let spell = CardDefinitionBuilder::new(CardId::new(), "Matching Card")
            .card_types(vec![T::Instant])
            .mana_cost(ironsmith::mana::ManaCost::from_symbols(vec![
                ironsmith::mana::ManaSymbol::Blue,
            ]))
            .with_spell_effect(vec![ironsmith::Effect::draw(1)])
            .build();
        let spell_id = game.create_object_from_definition(&spell, p, Zone::Hand);
        let cast = compute_legal_actions(&game, p)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::CastSpell { spell_id: id, .. } if *id == spell_id))
            .expect("spell castable");
        drive(&mut game, &mut queue, cast, |g, s| {
            s.pending_cast.is_none() && !g.stack.is_empty()
        });
        game.turn.priority_player = Some(p);
        let activate = compute_legal_actions(&game, p)
            .unwrap()
            .into_iter()
            .find(|a| matches!(a, LegalAction::ActivateAbility { source: s, .. } if *s == source))
            .expect("Jester's Scepter activation offered");
        drive(&mut game, &mut queue, activate, |g, s| {
            s.pending_activation.is_none() && g.stack.len() == 2
        });
        resolve_top(&mut game, &mut queue);
        let on_stack = game.stack.iter().any(|entry| {
            game.object(entry.object_id)
                .is_some_and(|o| o.name == "Matching Card")
        });
        !on_stack
    }

    #[test]
    fn jesters_scepter_counters_only_a_spell_named_like_the_paid_card() {
        on_big_stack(|| {
            assert!(run("Matching Card"), "same-named spell must be countered");
            assert!(
                !run("Other Card"),
                "differently named spell must stay on the stack"
            );
        });
    }
}

/// Deck validation reads these typed values; the label text is presentation.
#[test]
fn commander_pairing_and_copy_limits_are_typed() {
    use ironsmith::static_abilities::CommanderPairing;
    on_big_stack(|| {
        let statics = |name: &str| {
            compile(name)
                .abilities
                .into_iter()
                .filter_map(|ability| match ability.kind {
                    AbilityKind::Static(s) => Some(s),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let pairing = |name: &str| statics(name).iter().find_map(|s| s.commander_pairing());
        assert_eq!(
            pairing("Thrasios, Triton Hero"),
            Some(CommanderPairing::Partner)
        );
        assert_eq!(
            pairing("Pir, Imaginative Rascal"),
            Some(CommanderPairing::PartnerWith(
                "toothy, imaginary friend".into()
            ))
        );
        assert_eq!(
            pairing("Wilson, Refined Grizzly"),
            Some(CommanderPairing::ChooseBackground)
        );
        assert!(matches!(
            pairing("Abby, Merciless Soldier"),
            Some(CommanderPairing::Group(group)) if group.contains("survivors")
        ));
        let limit = |name: &str| statics(name).iter().find_map(|s| s.deck_copy_limit());
        assert_eq!(limit("Relentless Rats"), Some(None));
        assert_eq!(limit("Seven Dwarves"), Some(Some(7)));
    });
}

#[test]
fn cast_time_counts_preserve_selection_set_constraints() {
    on_big_stack(|| {
        let def = compile("Augur of Autumn");
        let condition = static_condition(&def);
        for (powers, expected) in [([2, 2], false), ([1, 3], true)] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            let player = PlayerId::from_index(0);
            let source = game.create_object_from_definition(&def, player, Zone::Battlefield);
            for (i, power) in powers.into_iter().enumerate() {
                game.create_object_from_definition(
                    &creature(&format!("Witness {i}"), power),
                    player,
                    Zone::Battlefield,
                );
            }
            game.refresh_continuous_state().unwrap();
            assert_eq!(
                ironsmith::condition_eval::evaluate_condition_cast_time_checked(
                    &game, &condition, player, source
                )
                .unwrap(),
                expected,
                "powers {powers:?}"
            );
        }
    });
}

#[test]
fn audit_graveyard_permissions_describe_nested_cost_conditions() {
    use ironsmith_core::ThisSpellCostCondition as C;
    on_big_stack(|| {
        let base = witness("Conditional Permission", vec![T::Creature]);
        let render = |condition, filter| {
            let grant = ironsmith_core::GrantSpec::new(
                ironsmith_core::Grantable::graveyard_cast_from_cards_mana_cost_with_condition(
                    condition, false,
                ),
                filter,
                Zone::Graveyard,
            );
            let mut def = base.clone();
            def.abilities
                .push(ironsmith::ability::Ability::static_ability(
                    StaticAbility::from_model(
                        ironsmith::static_abilities::CompiledStaticAbility::grants(grant),
                    ),
                ));
            audit_lines(&def).join("\n")
        };
        for (before, after, first, second) in [
            (
                C::YouLifeTotalOrLess(5),
                C::YouLifeTotalOrLess(15),
                "5",
                "15",
            ),
            (
                C::All(vec![C::YourTurn, C::YouLifeTotalOrLess(5)]),
                C::All(vec![C::YourTurn, C::YouLifeTotalOrLess(15)]),
                "5",
                "15",
            ),
            (
                C::OpponentControlsLandsOrMore(3),
                C::OpponentControlsLandsOrMore(7),
                "3",
                "7",
            ),
        ] {
            for filter in [
                ironsmith::target::ObjectFilter::source(),
                ironsmith::target::ObjectFilter::creature(),
            ] {
                let a = render(before.clone(), filter.clone());
                let b = render(after.clone(), filter);
                assert_ne!(a, b);
                assert!(a.contains(first), "{a}");
                assert!(b.contains(second), "{b}");
                assert!(!a.contains("the condition is true"));
            }
        }
    });
}

#[test]
fn snapshot_scoring_uses_semantics_even_when_display_hides_a_mutation() {
    on_big_stack(|| {
        let def = compile("Bruvac the Grandiloquent");
        let (mutated, any) = map_static_models(&def, |model| {
            if let P::EventAmountReplacement { modifier, .. } = &mut model.payload {
                *modifier = ironsmith_core::AmountModifierSpec::SetTo(9);
                true
            } else {
                false
            }
        });
        assert!(any);
        let display = ironsmith::compiled_text::compiled_text_lines(&def);
        assert_eq!(
            display,
            ironsmith::compiled_text::compiled_text_lines(&mutated)
        );
        let oracle = display.join("\n");
        let snapshot = ironsmith_tools::CompilationSnapshot::from_definition_result(
            "Bruvac the Grandiloquent",
            &oracle,
            ironsmith_tools::ParseStatus::StrictCompiled,
            None,
            Some(&mutated),
            &Default::default(),
        );
        let audit = audit_lines(&mutated);
        let expected = ironsmith::semantic_compare::compare_card_semantics_scored(
            "Bruvac the Grandiloquent",
            &snapshot.normalized_oracle_text,
            &audit,
            ironsmith::semantic_compare::report_embedding_config(),
        );
        assert_eq!(snapshot.similarity_score, expected.2);
        assert!(
            snapshot.similarity_score < 1.0,
            "stale correct-looking prose must not score exactly"
        );
        assert_eq!(snapshot.compiled_text.as_deref(), Some(oracle.as_str()));
    });
}

#[test]
fn corpus_discovered_static_prose_is_structural_or_explicitly_unverified() {
    on_big_stack(|| {
        for (name, unverified) in [
            ("Elephant Grass", false),
            ("Earth", false),
            ("Chaos Moon", false),
            ("Surveillance Phantasm", false),
            ("Shatterskull Charger", false),
            ("Rhet-Crop Spearmaster", false),
            ("Hammerhead Shark", false),
            ("Embermaw Hellion", false),
            ("Etching of Kumano", false),
            ("Titania's Song", false),
            ("Cogwork Tracker", true),
            ("Plague Nurse", true),
            ("Lore Seeker", true),
            ("Elemental Time Flamingo", true),
        ] {
            let def = compile(name);
            let before = audit_lines(&def);
            if unverified {
                let snapshot = ironsmith_tools::CompilationSnapshot::from_definition_result(
                    name,
                    &def.canonical_text,
                    ironsmith_tools::ParseStatus::StrictCompiled,
                    None,
                    Some(&def),
                    &Default::default(),
                );
                assert!(
                    snapshot.semantic_mismatch,
                    "{name}: unverified must be flagged"
                );
            }
            for (_, kind, changed) in semantic_audit_probes::executable_mutations(&def) {
                assert_ne!(before, audit_lines(&changed), "{name}: hidden {kind}");
            }
            assert_eq!(before, audit_lines(&poison_definition(&def)), "{name}");
            assert_eq!(
                before.join("\n").contains(UNVERIFIED_MARKER_PREFIX),
                unverified,
                "{name}"
            );
        }
    });
}

#[test]
fn aftermath_audit_reads_the_permission_zone_instead_of_its_label() {
    on_big_stack(|| {
        let def = compile("Earth");
        let before = audit_lines(&def);
        let (after, changed) = map_static_models(&def, |model| {
            if let P::Grants(grant) = &mut model.payload {
                grant.zone = Zone::Exile;
                true
            } else {
                false
            }
        });
        assert!(changed);
        assert_ne!(before, audit_lines(&after));
    });
}

#[test]
fn audit_turn_history_keeps_thresholds_and_counter_types() {
    on_big_stack(|| {
        use ironsmith_core::{CounterType, PlayerFilter, TurnHistoryCount as H, Value};
        let base = compile("Surveillance Phantasm");
        let render = |query: H, threshold: i32| {
            let (changed, found) = map_static_models(&base, |model| {
                let P::Conditional { condition, .. } = &mut model.payload else {
                    return false;
                };
                *condition = ConditionExpr::ValueComparison {
                    left: Value::TurnHistoryCount(query.clone()),
                    operator: ironsmith::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(threshold),
                };
                true
            });
            assert!(found);
            audit_lines(&changed)
        };
        let energy = H::PlayerCountersRemoved {
            player: PlayerFilter::You,
            counter_type: Some(CounterType::Energy),
        };
        let poison = H::PlayerCountersRemoved {
            player: PlayerFilter::You,
            counter_type: Some(CounterType::Poison),
        };
        assert_ne!(render(energy.clone(), 1), render(energy.clone(), 8));
        assert_ne!(render(energy, 1), render(poison, 1));
        let counters = H::CountersRemovedFrom {
            counter_type: Some(CounterType::Oil),
            filter: ironsmith::target::ObjectFilter::default(),
        };
        assert_ne!(render(counters.clone(), 1), render(counters, 8));
        let composed = |action| {
            let leaf = ConditionExpr::ValueComparison {
                left: Value::TurnHistoryCount(H::KeywordActionsPerformed {
                    player: PlayerFilter::You,
                    actions: vec![action],
                }),
                operator: ironsmith::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            };
            let logical_tree = ConditionExpr::And(
                Box::new(leaf.clone()),
                Box::new(ConditionExpr::And(Box::new(leaf.clone()), Box::new(leaf))),
            );
            let (changed, found) = map_static_models(&base, |model| {
                let P::Conditional { condition, .. } = &mut model.payload else {
                    return false;
                };
                *condition = logical_tree.clone();
                true
            });
            assert!(found);
            audit_lines(&changed)
        };
        assert_ne!(
            composed(ironsmith_core::KeywordActionKind::Scry),
            composed(ironsmith_core::KeywordActionKind::Surveil)
        );
    });
}
