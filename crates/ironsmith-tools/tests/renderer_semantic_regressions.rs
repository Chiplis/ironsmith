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
    let payloads =
        ironsmith_tools::load_card_payloads_by_names("../../cards.json", &[name.to_string()])
            .unwrap();
    ironsmith_tools::compile_runtime_definition_from_payload(
        payloads.values().next().unwrap().first().unwrap(),
    )
    .unwrap()
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
            (&[&[T::Creature], &[T::Creature], &[T::Artifact], &[T::Land]], false),
            (&[&[T::Creature], &[T::Creature], &[T::Creature], &[T::Creature]], false),
            (&[&[T::Artifact, T::Creature], &[T::Enchantment], &[T::Land]], true),
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
            let host = game.create_object_from_definition(&creature("Host", 2), p, Zone::Battlefield);
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
    let reference = game.create_object_from_definition(&witness("Matching Card", vec![ty]), p, ref_zone);
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
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(reference).unwrap(), &game),
        );
    }
    ironsmith::condition_eval::evaluate_condition_resolution(&game, condition, &ctx).unwrap()
}

#[test]
fn resolution_same_name_counts_keep_their_bound_reference() {
    on_big_stack(|| {
        for (name, ty, ref_zone, it_zone, negated) in [
            ("Candles of Leng", T::Sorcery, Zone::Graveyard, Zone::Stack, false),
            ("Retraced Image", T::Creature, Zone::Battlefield, Zone::Hand, false),
            ("Guardian Project", T::Creature, Zone::Graveyard, Zone::Battlefield, true),
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
        let condition = resolution_conditions(&def).into_iter().next().expect("gate");
        let debug = format!("{condition:?}");
        assert!(debug.contains("zone_cost_0"), "paid card is its own operand: {debug}");
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

/// Replace retained presentation prose in a condition with POISON.
fn poison_condition(condition: &mut ConditionExpr) {
    match condition {
        ConditionExpr::CountComparison { display, .. }
        | ConditionExpr::CountParity { display, .. } => {
            if display.is_some() {
                *display = Some(POISON.into());
            }
        }
        ConditionExpr::AttachmentCount { display, .. } => *display = POISON.into(),
        ConditionExpr::Not(inner) => poison_condition(inner),
        ConditionExpr::And(left, right) | ConditionExpr::Or(left, right) => {
            poison_condition(left);
            poison_condition(right);
        }
        _ => {}
    }
}

const POISON: &str = "POISON";

/// Replace retained presentation prose in every display-bearing static
/// payload field (`display`, `*_surface`, and the displays of nested
/// conditions). Arms were generated from every `StaticAbilityPayload` variant
/// with such a field; extend them when a variant gains one.
fn poison_model(model: &mut ironsmith::static_abilities::CompiledStaticAbility) -> bool {
    // The label is kept: it carries ability words ("Coven —"), which have no
    // rules meaning, and the unrendered-model fallback is already marked.
    match &mut model.payload {
        P::Anthem(anthem) => anthem.condition.iter_mut().for_each(poison_condition),
        P::AttachedAbilityGrant(grant) => {
            grant.display = POISON.into();
            grant.condition.iter_mut().for_each(poison_condition);
        }
        P::GrantObjectAbilityForFilter(grant) => {
            if !grant.derived_ability_display {
                grant.display = POISON.into();
            }
            grant.condition.iter_mut().for_each(poison_condition);
        }
        P::Grants(grant) => {
            if grant.filtered_zone_surface.is_some() {
                grant.filtered_zone_surface = Some(POISON.into());
            }
        }
        P::CountersRemainAcrossZoneChanges { display, .. } => { *display = POISON.into(); }
        P::AnyPlayerMayPayManaToIgnoreSourceEffectUntilEndOfTurn { display, .. } => { *display = POISON.into(); }
        P::Conditional { condition, .. } => { poison_condition(condition); }
        P::CostIncreaseLife { display, .. } => { *display = POISON.into(); }
        P::ActivateAbilitiesAsThoughHaste { display, .. } => { *display = POISON.into(); }
        P::ThisSpellCastRestriction { display, .. } => { *display = POISON.into(); }
        P::ThisSpellXMaximum { display, .. } => { *display = POISON.into(); }
        P::ThisSpellXMinimum { display, .. } => { *display = POISON.into(); }
        P::RuleRestriction { display, .. } => { *display = POISON.into(); }
        P::CounterLimit { display, .. } => { *display = POISON.into(); }
        P::CantAttackUnlessCondition { display, .. } => { *display = POISON.into(); }
        P::AttackCost { display, .. } => { *display = POISON.into(); }
        P::BlockCost { display, .. } => { *display = POISON.into(); }
        P::UntapDuringEachOtherPlayersUntapStep { display, .. } => { *display = POISON.into(); }
        P::UntapStepLimit { display, .. } => { *display = POISON.into(); }
        P::UntapStepChosenCardTypeOnly { display, .. } => { *display = POISON.into(); }
        P::SearchLimitedToTopCards { display, .. } => { *display = POISON.into(); }
        P::PlayerProtectionFrom { display, .. } => { *display = POISON.into(); }
        P::AddChosenCreatureType { display, .. } => { *display = POISON.into(); }
        P::AddChosenBasicLandType { display, .. } => { *display = POISON.into(); }
        P::SourceHasAbilityStickerAbilitiesOf { display, .. } => { *display = POISON.into(); }
        P::AddChosenColor { display, .. } => { *display = POISON.into(); }
        P::SetChosenColor { display, .. } => { *display = POISON.into(); }
        P::DuplicateMatchingTriggeredAbilities { display, .. } => { *display = POISON.into(); }
        P::SuppressMatchingTriggeredAbilities { display, .. } => { *display = POISON.into(); }
        P::ExertAttack { display, .. } => { *display = POISON.into(); }
        P::EnlistAttack { display, .. } => { *display = POISON.into(); }
        P::RemoveObjectAbilitiesForFilter { display, .. } => { *display = POISON.into(); }
        P::RemoveCardTypes { condition, .. } => { condition.iter_mut().for_each(poison_condition); }
        P::ActivatedAbilityCostIncrease { condition, display, .. } => { condition.iter_mut().for_each(poison_condition); if display.is_some() { *display = Some(POISON.into()); } }
        P::ChooseColorAsEnters { display, .. } => { *display = POISON.into(); }
        P::ChoosePlayerAsEnters { display, .. } => { *display = POISON.into(); }
        P::RevealFromHandAsEnters { display, .. } => { *display = POISON.into(); }
        P::ChooseCardNameAsEnters { display, .. } => { *display = POISON.into(); }
        P::ChooseNamedOptionAsEnters { display, .. } => { *display = POISON.into(); }
        P::ChoosePowerToughnessAsEntersOrTurnsFaceUp { display, .. } => { *display = POISON.into(); }
        P::EnterAsCopyAsEnters { display, .. } => { *display = POISON.into(); }
        P::RedirectDamageToSourceController { display, .. } => { *display = POISON.into(); }
        P::ExileWouldDieInstead { damager_filter_surface, .. } => { if damager_filter_surface.is_some() { *damager_filter_surface = Some(POISON.into()); } }
        P::ModifyDamageAmountReplacement { display, .. } => { *display = POISON.into(); }
        P::MinimumDamageAmountReplacement { display, .. } => { *display = POISON.into(); }
        P::DoubleDamageAmountReplacement { display, .. } => { *display = POISON.into(); }
        P::PreventHalfDamageReplacement { display, .. } => { *display = POISON.into(); }
        P::DoubleCountersReplacement { display, .. } => { *display = POISON.into(); }
        P::AddCountersPlacementReplacement { display, .. } => { *display = POISON.into(); }
        P::PlayerCounterPerTurnLimitReplacement { display, .. } => { *display = POISON.into(); }
        P::DoubleTokenCreationReplacement { display, .. } => { *display = POISON.into(); }
        P::MultiplyTokenCreationReplacement { display, .. } => { *display = POISON.into(); }
        P::AddTokenCreationReplacement { display, .. } => { *display = POISON.into(); }
        P::KeywordActionReplacement { display, .. } => { *display = POISON.into(); }
        P::ConditionalDrawReplacement { condition, display, .. } => { poison_condition(condition); *display = POISON.into(); }
        P::CreateOneOfEachTokenReplacement { display, .. } => { *display = POISON.into(); }
        P::RedirectDrawReplacement { display, .. } => { *display = POISON.into(); }
        P::DrawReplacementWithEffects { display, .. } => { *display = POISON.into(); }
        P::DrawExtraCardsReplacement { display, .. } => { *display = POISON.into(); }
        P::LoseGameReplacement { display, .. } => { *display = POISON.into(); }
        P::DrawReplacementRevealTopMatchingToHandRestBottom { display, .. } => { *display = POISON.into(); }
        P::RedirectWouldEnter { display, .. } => { *display = POISON.into(); }
        P::ManaProductionReplacement { display, .. } => { *display = POISON.into(); }
        P::ManaProductionMultiplierReplacement { display, .. } => { *display = POISON.into(); }
        P::DoubleLifeChangeReplacement { display, .. } => { *display = POISON.into(); }
        P::ManaSpendPermission { display, .. } => { *display = POISON.into(); }
        P::PreventDamageToSelfPutCountersInstead { display, .. } => { *display = POISON.into(); }
        P::HealDamageBeforeDamageToSelf { display, .. } => { *display = POISON.into(); }
        P::PreventConstrainedDamageToSelfPutCountersInstead { display, .. } => { *display = POISON.into(); }
        P::PreventDamageToYouFromSourceFilter { display, .. } => { *display = POISON.into(); }
        P::ReplaceDamageWithCountersInstead { display, .. } => { *display = POISON.into(); }
        P::EntersTappedUnlessCondition { condition, display, .. } => { poison_condition(condition); *display = POISON.into(); }
        P::EntersWithCountersIfCondition { condition, display, .. } => { poison_condition(condition); *display = POISON.into(); }
        P::EntersWithCounterChoice { condition, display, .. } => { condition.iter_mut().for_each(poison_condition); *display = POISON.into(); }
        P::AddLifeGainReplacement { display, .. } => { *display = POISON.into(); }
        P::TokenCreationTemplates { display, .. } => { *display = POISON.into(); }
        P::ActorCountersAddition { display, .. } => { *display = POISON.into(); }
        P::ManaProductionRewrite { display, .. } => { *display = POISON.into(); }
        P::EventReplacementWithEffects { display, .. } => { *display = POISON.into(); }
        P::EventAmountReplacement { display, .. } => { *display = POISON.into(); }
        P::EchoCostAlternative { display, .. } => { *display = POISON.into(); }
        P::SourceHasFullTextOfTopGraveyardCard { display, .. } => { *display = POISON.into(); }
        P::MaySkipDrawStepWithFollowup { display, .. } => { *display = POISON.into(); }
        P::PlayerMaySacrificePermanentToIgnoreSourceEffectUntilEndOfTurn { display, .. } => { *display = POISON.into(); }
        P::CastTargetRestriction { display, .. } => { *display = POISON.into(); }
        P::EntersAttachedToChosenObject { display, .. } => { *display = POISON.into(); }
        P::AdditionalSacrificePerManaSymbol { display, .. } => { *display = POISON.into(); }
        P::ControllerCounterExilePermission { display, .. } => { *display = POISON.into(); }
        P::AttackingPlayerChoosesBlocks { display, .. } => { *display = POISON.into(); }
        P::AttachedControllerMayPayCostToIgnoreSourceEffectUntilEndOfTurn { display, .. } => { *display = POISON.into(); }
        P::ChooseBasicLandTypesAsEnters { display, .. } => { *display = POISON.into(); }
        P::FirstChosenLandTypeBecomesSecond { display, .. } => { *display = POISON.into(); }
        P::DamageBetweenChosenPlayers { display, .. } => { *display = POISON.into(); }
        P::AddSubtypesAndGrantAbilities { display, .. } => { *display = POISON.into(); }
        _ => {}
    }
    true
}

fn map_static_models(
    def: &CardDefinition,
    mut f: impl FnMut(&mut ironsmith::static_abilities::CompiledStaticAbility) -> bool,
) -> (CardDefinition, bool) {
    let mut changed = def.clone();
    let mut any = false;
    for ability in &mut changed.abilities {
        let AbilityKind::Static(s) = &ability.kind else {
            continue;
        };
        let Some(model) = s.compiled_model() else {
            continue;
        };
        let mut model = model.clone();
        if f(&mut model) {
            any = true;
            ability.kind = AbilityKind::Static(StaticAbility::from_model(model));
        }
    }
    (changed, any)
}

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

fn poison_effect(effect: ironsmith::Effect) -> ironsmith::Effect {
    use ironsmith::effects as fx;
    let poison_all = |effects: &mut Vec<ironsmith::Effect>| {
        *effects = std::mem::take(effects).into_iter().map(poison_effect).collect();
    };
    if let Some(conditional) = effect.downcast_ref::<fx::ConditionalEffect>() {
        let mut conditional = conditional.clone();
        poison_condition(&mut conditional.condition);
        poison_all(&mut conditional.if_true);
        poison_all(&mut conditional.if_false);
        return ironsmith::Effect::new(conditional);
    }
    if let Some(with_id) = effect.downcast_ref::<fx::WithIdEffect>() {
        let mut with_id = with_id.clone();
        with_id.effect = Box::new(poison_effect(*with_id.effect));
        return ironsmith::Effect::new(with_id);
    }
    if let Some(tagged) = effect.downcast_ref::<fx::TaggedEffect>() {
        let mut tagged = tagged.clone();
        tagged.effect = Box::new(poison_effect(*tagged.effect));
        return ironsmith::Effect::new(tagged);
    }
    if let Some(sequence) = effect.downcast_ref::<fx::SequenceEffect>() {
        let mut sequence = sequence.clone();
        poison_all(&mut sequence.effects);
        return ironsmith::Effect::new(sequence);
    }
    if let Some(may) = effect.downcast_ref::<fx::MayEffect>() {
        let mut may = may.clone();
        poison_all(&mut may.effects);
        return ironsmith::Effect::new(may);
    }
    if let Some(for_players) = effect.downcast_ref::<fx::ForPlayersEffect>() {
        let mut for_players = for_players.clone();
        poison_all(&mut for_players.effects);
        return ironsmith::Effect::new(for_players);
    }
    effect
}

fn poison_program(program: &mut ironsmith::resolution::ResolutionProgram) {
    let original = std::mem::take(program);
    *program = original
        .try_map_effects(|effect| Ok::<_, std::convert::Infallible>(poison_effect(effect)))
        .expect("infallible poison transform");
}

/// Poison every presentation field the audit renderer must not print:
/// static payloads, intervening-if conditions, and resolution conditions.
fn poison_definition(def: &CardDefinition) -> CardDefinition {
    let (mut poisoned, _) = map_static_models(def, poison_model);
    if let Some(program) = &mut poisoned.spell_effect {
        poison_program(program);
    }
    for ability in &mut poisoned.abilities {
        match &mut ability.kind {
            AbilityKind::Triggered(triggered) => {
                triggered.intervening_if.iter_mut().for_each(poison_condition);
                poison_program(&mut triggered.effects);
            }
            AbilityKind::Activated(activated) => poison_program(&mut activated.effects),
            AbilityKind::Static(_) => {}
        }
    }
    poisoned
}

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
        assert!(leaks.is_empty(), "audit rendering depends on presentation text:\n{}", leaks.join("\n"));
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
                    *increase = ironsmith_core::TotalCost::mana(
                        ironsmith::mana::ManaCost::from_symbols(vec![
                            ironsmith::mana::ManaSymbol::Generic(9),
                        ]),
                    );
                    true
                }
                P::Grants(grant) if grant.filtered_zone_surface.is_some() => {
                    grant.zone = Zone::Graveyard;
                    true
                }
                _ => false,
            });
            assert!(any, "{name}: no executable field was mutated");
            assert_ne!(audit_lines(&mutated), before, "{name}: audit hid a semantic change");
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
        let exiled = game.create_object_from_definition(&witness(paid_name, vec![T::Instant]), p, Zone::Exile);
        game.add_exiled_with_source_link(source, exiled);
        for symbol in [ironsmith::mana::ManaSymbol::Blue, ironsmith::mana::ManaSymbol::Colorless] {
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
        drive(&mut game, &mut queue, cast, |g, s| s.pending_cast.is_none() && !g.stack.is_empty());
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
        let on_stack = game
            .stack
            .iter()
            .any(|entry| game.object(entry.object_id).is_some_and(|o| o.name == "Matching Card"));
        !on_stack
    }

    #[test]
    fn jesters_scepter_counters_only_a_spell_named_like_the_paid_card() {
        on_big_stack(|| {
            assert!(run("Matching Card"), "same-named spell must be countered");
            assert!(!run("Other Card"), "differently named spell must stay on the stack");
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
        assert_eq!(pairing("Thrasios, Triton Hero"), Some(CommanderPairing::Partner));
        assert_eq!(
            pairing("Pir, Imaginative Rascal"),
            Some(CommanderPairing::PartnerWith("toothy, imaginary friend".into()))
        );
        assert_eq!(pairing("Wilson, Refined Grizzly"), Some(CommanderPairing::ChooseBackground));
        assert!(matches!(
            pairing("Abby, Merciless Soldier"),
            Some(CommanderPairing::Group(group)) if group.contains("survivors")
        ));
        let limit = |name: &str| statics(name).iter().find_map(|s| s.deck_copy_limit());
        assert_eq!(limit("Relentless Rats"), Some(None));
        assert_eq!(limit("Seven Dwarves"), Some(Some(7)));
    });
}
