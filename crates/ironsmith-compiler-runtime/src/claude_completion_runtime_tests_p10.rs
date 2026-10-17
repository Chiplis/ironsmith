//! Claude package 10 runtime checks for the compilation-completion task
//! (see reports/compilation-completion-20261016/COORDINATION.md).
#![allow(unused_imports)]
use super::compile_to_runtime_definition;
use ironsmith::effect::Effect;

fn p10_visit(effect: &Effect, f: &mut dyn FnMut(&Effect)) {
    f(effect);
    effect.visit_child_effects(&mut |child| p10_visit(child, f));
}

/// Every effect reachable from a definition (spell, activated, triggered).
fn p10_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<Effect> {
    let mut out = Vec::new();
    let mut push = |effect: &Effect| out.push(effect.clone());
    if let Some(program) = definition.spell_effect.as_ref() {
        for effect in program.all_effects() { p10_visit(effect, &mut push); }
    }
    for ability in &definition.abilities {
        match &ability.kind {
            ironsmith::ability::AbilityKind::Activated(activated) => {
                for effect in activated.effects.all_effects() { p10_visit(effect, &mut push); }
            }
            ironsmith::ability::AbilityKind::Triggered(triggered) => {
                for effect in triggered.effects.all_effects() { p10_visit(effect, &mut push); }
            }
            _ => {}
        }
    }
    out
}

const P10_MIZZIX: &str = "Mana cost: {4}{R}\nType: Legendary Creature — Goblin Wizard\nFlying\nWhenever you cast a spell from anywhere other than your hand, you may pay {1}{U/R}. If you do, copy that spell and you may choose new targets for the copy. If the copy is a permanent spell, it gains haste and \"At the beginning of your end step, sacrifice this permanent.\" (A copy of a permanent spell becomes a token.)";
const P10_SPIDER_VERSE: &str = "Mana cost: {3}{R}{R}\nType: Enchantment\nThe \"legend rule\" doesn't apply to Spiders you control.\nWhenever you cast a spell from anywhere other than your hand, you may copy it. If you do, you may choose new targets for the copy. If the copy is a permanent spell, it gains haste. Do this only once each turn.";

#[test]
fn the_copy_is_a_permanent_spell_tests_and_grants_to_the_created_copy() {
    for (name, text) in [("Mizzix, Replica Rider", P10_MIZZIX), ("Spider-Verse", P10_SPIDER_VERSE)] {
        let definition = compile_to_runtime_definition(name, text, false)
            .unwrap_or_else(|error| panic!("{name} should compile: {error:?}"));
        let effects = p10_effects(&definition);
        let copy_tag = effects.iter().find_map(|effect| {
            let tagged = effect.downcast_ref::<ironsmith::effects::TaggedEffect>()?;
            let mut inner = &*tagged.effect;
            while let Some(with_id) = inner.downcast_ref::<ironsmith::effects::WithIdEffect>() { inner = &with_id.effect; }
            inner.downcast_ref::<ironsmith::effects::CopySpellEffect>().map(|_| tagged.tag.clone())
        }).unwrap_or_else(|| panic!("{name}: the copy instruction must publish its copy"));
        let conditional = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ConditionalEffect>().cloned())
            .unwrap_or_else(|| panic!("{name}: conditional on the copy"));
        match &conditional.condition {
            ironsmith::effect::Condition::TaggedObjectMatches(tag, filter) => {
                assert_eq!(tag, &copy_tag, "{name}: the predicate tests the created copy");
                assert_eq!(filter.zone, Some(ironsmith::Zone::Stack), "{name}");
            }
            other => panic!("{name}: unexpected condition {other:?}"),
        }
        let grant = conditional.if_true.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ApplyContinuousEffect>().cloned())
            .unwrap_or_else(|| panic!("{name}: the copy gains haste"));
        assert!(matches!(&grant.target_spec, Some(ironsmith::target::ChooseSpec::Tagged(tag)) if tag == &copy_tag),
            "{name}: `it` is the copy: {:?}", grant.target_spec);
        let debug = format!("{grant:?}");
        assert!(debug.contains("Haste"), "{name}: {debug}");
        if name == "Mizzix, Replica Rider" {
            assert!(debug.contains("SacrificeTargetEffect") || debug.contains("Sacrifice"), "{name}: granted end-step sacrifice: {debug}");
        }
    }

    // "The legend rule doesn't apply to Spiders you control" keeps its type.
    let spider_verse = compile_to_runtime_definition("Spider-Verse", P10_SPIDER_VERSE, false).unwrap();
    let legend = spider_verse.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Static(rule)
            if rule.id() == ironsmith::static_abilities::StaticAbilityId::LegendRuleDoesntApplyToController => Some(format!("{rule:?}")),
        _ => None,
    }).expect("legend-rule exemption");
    assert!(legend.contains("Spider"), "the exemption is for Spiders only: {legend}");
}

#[test]
fn enchanted_creature_is_a_copy_of_the_creature_chosen_as_the_aura_entered() {
    use ironsmith::card::{CardBuilder, PowerToughness};
    use ironsmith::ids::CardId;
    use ironsmith::types::CardType;
    use ironsmith::{PlayerId, Zone};
    let definition = compile_to_runtime_definition("Metamorphic Alteration",
        "Mana cost: {1}{U}\nType: Enchantment — Aura\nEnchant creature\nAs this Aura enters, choose a creature.\nEnchanted creature is a copy of the chosen creature.", false)
        .expect("an Aura copy static keyed to an as-enters choice should compile");
    let debug = format!("{:?}", definition.abilities);
    assert!(debug.contains("remember_as_chosen_object: true"),
        "the as-enters choice must persist for the later static: {debug}");
    assert!(definition.abilities.iter().any(|ability| matches!(&ability.kind,
        ironsmith::ability::AbilityKind::Static(rule)
            if rule.id() == ironsmith::static_abilities::StaticAbilityId::AttachedIsCopyOfChosenObject)));

    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let bear = CardBuilder::new(CardId::new(), "Bear").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2)).build();
    let giant = CardBuilder::new(CardId::new(), "Giant").card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(7, 7)).build();
    let host = game.create_object_from_card(&bear, bob, Zone::Battlefield);
    let chosen = game.create_object_from_card(&giant, alice, Zone::Battlefield);
    let aura = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(aura, ironsmith::object::AttachmentTarget::Object(host)));
    let snapshot = ironsmith::snapshot::ObjectSnapshot::from_object(game.object(chosen).unwrap(), &game);
    game.set_chosen_object(aura, snapshot);
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.calculated_power(host), Some(7), "the host copies the chosen creature");
    assert_eq!(&*game.calculated_characteristics(host).unwrap().name, "Giant");
    assert_eq!(game.calculated_power(chosen), Some(7));
}

#[test]
fn search_for_a_card_with_the_same_name_binds_the_chosen_permanent() {
    let definition = compile_to_runtime_definition("Mimeofacture",
        "Mana cost: {3}{U}\nType: Sorcery\nReplicate {3}{U} (When you cast this spell, copy it for each time you paid its replicate cost. You may choose new targets for the copies.)\nChoose target permanent an opponent controls. Search that player's library for a card with the same name and put it onto the battlefield under your control. Then that player shuffles.", false)
        .expect("implicit same-name search should compile");
    let effects = p10_effects(&definition);
    let target_tag = effects.iter().find_map(|effect| {
        let tagged = effect.downcast_ref::<ironsmith::effects::TaggedEffect>()?;
        tagged.effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().map(|_| tagged.tag.clone())
    }).expect("the chosen target is tagged");
    let search = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ChooseObjectsEffect>().cloned())
        .expect("library search");
    assert!(search.filter.tagged_constraints.iter().any(|constraint|
        constraint.tag == target_tag && constraint.relation == ironsmith::filter::TaggedOpbjectRelation::SameNameAsTagged),
        "the searched card shares the chosen permanent's name: {:?}", search.filter.tagged_constraints);
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::PutOntoBattlefieldEffect>().is_some()));
}

#[test]
fn they_entered_or_were_cast_from_a_graveyard_scopes_the_batch_trigger() {
    let definition = compile_to_runtime_definition("Twilight Diviner",
        "Mana cost: {2}{B}\nType: Creature — Elf Cleric\nPower/Toughness: 3/1\nWhen this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)\nWhenever one or more other creatures you control enter, if they entered or were cast from a graveyard, create a token that's a copy of one of them. This ability triggers only once each turn.", false)
        .expect("shared moved-or-cast origin should compile");
    let triggers: Vec<_> = definition.abilities.iter().filter_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered),
        _ => None,
    }).collect();
    assert_eq!(triggers.len(), 2);
    let batch = format!("{:?}", triggers[1].trigger);
    assert!(batch.contains("MovedFromOrCastFrom") && batch.contains("zone: Graveyard"), "{batch}");
    assert!(batch.contains("OneOrMore"), "{batch}");
    assert!(format!("{:?}", triggers[1].intervening_if).contains("MaxTimesEachTurn(1)"));
    let copies = p10_effects(&definition).into_iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>().cloned())
        .collect::<Vec<_>>();
    assert_eq!(copies.len(), 1);
    assert!(format!("{:?}", copies[0].target).contains("triggering"), "copies one of the entering creatures");
}

#[test]
fn delayed_token_copy_enters_with_half_the_counters_the_dead_creature_had() {
    let definition = compile_to_runtime_definition("Ochre Jelly",
        "Mana cost: {X}{G}\nType: Creature — Ooze\nPower/Toughness: 0/0\nTrample\nThis creature enters with X +1/+1 counters on it.\nSplit — When this creature dies, if it had two or more +1/+1 counters on it, create a token that's a copy of it at the beginning of the next end step. The token enters with half that many +1/+1 counters on it, rounded down.", false)
        .expect("token copy entry counters should compile");
    let effects = p10_effects(&definition);
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>().is_some()),
        "the copy is created at the next end step");
    let copy = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>().cloned())
        .expect("token copy");
    let [counter] = copy.entry_counters.as_slice() else {
        panic!("the token enters with its counters: {:?}", copy.entry_counters);
    };
    assert_eq!(counter.counter_type, ironsmith::object::CounterType::PlusOnePlusOne);
    match counter.amount.unhinted() {
        ironsmith::effect::Value::HalfRoundedDown(inner) => assert!(
            matches!(inner.unhinted(), ironsmith::effect::Value::CountersOn(spec, Some(ironsmith::object::CounterType::PlusOnePlusOne))
                if matches!(spec.base(), ironsmith::target::ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")),
            "half of the dead creature's +1/+1 counters: {inner:?}"),
        other => panic!("rounded-down half: {other:?}"),
    }
    assert!(!effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::PutCountersEffect>().is_some()),
        "no separate put-counters action remains");
}

#[test]
fn two_graveyard_targets_are_exiled_copied_and_cast_if_able() {
    let definition = compile_to_runtime_definition("Spelltwine",
        "Mana cost: {5}{U}\nType: Sorcery\nExile target instant or sorcery card from your graveyard and target instant or sorcery card from an opponent's graveyard. Copy those cards. Cast the copies if able without paying their mana costs. Exile Spelltwine.", false)
        .expect("two-target graveyard exile with mandatory copy casts should compile");
    let effects = p10_effects(&definition);
    let target_exiles = effects.iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::ExileEffect>())
        .filter(|exile| exile.spec.is_target())
        .count();
    assert_eq!(target_exiles, 2, "one exile per graveyard target");
    let casts = effects.iter()
        .filter_map(|effect| effect.downcast_ref::<ironsmith::effects::CastTaggedEffect>().cloned())
        .collect::<Vec<_>>();
    assert!(!casts.is_empty());
    assert!(casts.iter().all(|cast| cast.as_copy && cast.without_paying_mana_cost));
    assert!(effects.iter().all(|effect| effect.downcast_ref::<ironsmith::effects::MayEffect>().is_none()),
        "'if able' casts are mandatory");
}

#[test]
fn next_entry_batch_enters_as_copies_of_the_chosen_creature() {
    use ironsmith::types::CardType;
    let definition = compile_to_runtime_definition("Mystic Reflection",
        "Mana cost: {1}{U}\nType: Instant\nChoose target nonlegendary creature. The next time one or more creatures or planeswalkers enter this turn, they enter as copies of the chosen creature.\nForetell {U} (During your turn, you may pay {2} and exile this card from your hand face down. Cast it on a later turn for its foretell cost.)", false)
        .expect("one-shot enter-as-copy registration should compile");
    let effects = p10_effects(&definition);
    let register = effects.iter()
        .find_map(|effect| effect.downcast_ref::<ironsmith::effects::RegisterNextBatchEnterAsCopyEffect>().cloned())
        .expect("next-batch enter-as-copy registration");
    let debug = format!("{:?}", register.filter);
    assert!(debug.contains("Creature") && debug.contains("Planeswalker"), "{debug}");
    assert!(!matches!(register.copy_source, ironsmith::target::ChooseSpec::Source),
        "copies the chosen creature, not the spell: {:?}", register.copy_source);

    // Registration installs a batch one-shot entry replacement.
    let alice = ironsmith::PlayerId::from_index(0);
    let mut game = ironsmith::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let giant = ironsmith::card::CardBuilder::new(ironsmith::ids::CardId::new(), "Giant")
        .card_types(vec![CardType::Creature])
        .power_toughness(ironsmith::card::PowerToughness::fixed(7, 7)).build();
    let giant = game.create_object_from_card(&giant, alice, ironsmith::Zone::Battlefield);
    let before = game.effect_store.replacement_effects.one_shot_effects_snapshot().len();
    let effect = Effect::new(ironsmith::effects::RegisterNextBatchEnterAsCopyEffect::new(
        register.filter.clone(), ironsmith::target::ChooseSpec::SpecificObject(giant)));
    let source = game.new_object_id();
    let mut ctx = ironsmith::effects::EffectContext::new_default(source, alice);
    ironsmith::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
    assert_eq!(game.effect_store.replacement_effects.one_shot_effects_snapshot().len(), before + 1);
}

#[test]
fn random_graveyard_exile_then_token_copy_repeats_while_the_exiled_card_is_a_land() {
    let definition = compile_to_runtime_definition("Sin, Spira's Punishment",
        "Mana cost: {4}{B}{G}{U}\nType: Legendary Creature — Leviathan Avatar\nPower/Toughness: 7/7\nFlying\nWhenever Sin enters or attacks, exile a permanent card from your graveyard at random, then create a tapped token that's a copy of that card. If the exiled card is a land card, repeat this process.", false)
        .expect("random exile, token copy, and land-conditioned repeat should compile");
    let effects = p10_effects(&definition);
    assert!(effects.iter().any(|effect| format!("{effect:?}").starts_with("Effect(RepeatProcessEffect")
        || effect.downcast_ref::<ironsmith::effects::RepeatProcessEffect>().is_some()), "repeat this process");
    let exile = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::ExileEffect>().cloned())
        .expect("random graveyard exile");
    assert!(exile.spec.count().random, "exiled at random: {:?}", exile.spec);
    let exile_debug = format!("{:?}", exile.spec);
    assert!(!exile_debug.contains("token: true") && !exile_debug.contains("tapped: true"),
        "the token creation must not leak into the exile filter: {exile_debug}");
    let copy = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::CreateTokenCopyEffect>().cloned())
        .expect("token copy of the exiled card");
    assert!(copy.enters_tapped);
}

#[test]
fn life_paid_to_activate_gates_and_sizes_the_repeat_payment() {
    let definition = compile_to_runtime_definition("Verrak, Warped Sengir",
        "Mana cost: {1}{W}{B}\nType: Legendary Creature — Vampire\nPower/Toughness: 4/4\nFlying, deathtouch, lifelink\nWhenever you activate an ability that isn't a mana ability, if life was paid to activate it, you may pay that much life again. If you do, copy that ability. You may choose new targets for the copy.", false)
        .expect("activation life-payment receipt should compile");
    let trigger = definition.abilities.iter().find_map(|ability| match &ability.kind {
        ironsmith::ability::AbilityKind::Triggered(triggered) => Some(triggered.clone()),
        _ => None,
    }).expect("activation trigger");
    let gate = format!("{:?}", trigger.intervening_if);
    assert!(gate.contains("LifePaidToActivate"), "{gate}");
    let effects = p10_effects(&definition);
    let pay = effects.iter().find_map(|effect| effect.downcast_ref::<ironsmith::effects::PayLifeEffect>().cloned())
        .expect("optional life payment");
    assert!(matches!(pay.amount.unhinted(), ironsmith::effect::Value::EventValue(ironsmith::effect::EventValueSpec::LifePaidToActivate)),
        "pays the life paid to activate: {:?}", pay.amount);
    assert!(effects.iter().any(|effect| effect.downcast_ref::<ironsmith::effects::CopySpellEffect>().is_some()));
}
