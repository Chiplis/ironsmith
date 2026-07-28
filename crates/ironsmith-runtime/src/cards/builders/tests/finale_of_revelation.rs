#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::effect::{Effect, Restriction, Until, Value};
use crate::effects::{
    CantEffect, DrawCardsEffect, SequenceEffect, ShuffleGraveyardIntoLibraryEffect,
};

fn unwrap_surface_wrappers(mut effect: &Effect) -> &Effect {
    loop {
        if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
            effect = &tagged.effect;
            continue;
        }
        if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
            effect = &with_id.effect;
            continue;
        }
        return effect;
    }
}

#[test]
fn resolving_no_maximum_hand_size_rule_lowers_generically() {
    let oracle = "You have no maximum hand size for the rest of the game.";
    let definition = CardDefinitionBuilder::new(CardId::new(), "Lasting hand-size rule")
        .card_types(vec![CardType::Sorcery])
        .parse_text(oracle)
        .expect("the resolving player rule should parse");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);
    let program = definition.spell_effect.as_ref().expect("spell resolution");
    let [effect] = program.segments[0].default_effects.as_slice() else {
        panic!("expected exactly one lasting rule effect: {program:#?}");
    };
    let cant = effect
        .downcast_ref::<CantEffect>()
        .expect("the rule should use the reusable lasting-restriction executor");
    assert_eq!(
        cant.restriction,
        Restriction::NoMaximumHandSize(PlayerFilter::You)
    );
    assert_eq!(cant.duration, Until::Forever);
}

#[test]
fn finale_of_revelation_preserves_one_ordered_replacement_body_and_exact_text() {
    let oracle = "Draw X cards. If X is 10 or more, instead shuffle your graveyard into your library, draw X cards, untap up to five lands, and you have no maximum hand size for the rest of the game.\nExile Finale of Revelation.";
    let definition = parse_oracle_card_definition("Finale of Revelation");

    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        oracle,
        "the full card must retain the authored replacement sequence"
    );

    let program = definition.spell_effect.as_ref().expect("spell resolution");
    assert_eq!(program.segments.len(), 2, "{program:#?}");
    let first = &program.segments[0];
    let [default_draw] = first.default_effects.as_slice() else {
        panic!("the normal branch must contain only Draw X: {first:#?}");
    };
    let draw = default_draw
        .downcast_ref::<DrawCardsEffect>()
        .expect("normal branch should draw cards");
    assert_eq!(draw.count, Value::X);

    let [branch] = first.self_replacements.as_slice() else {
        panic!("expected one X-at-least-ten replacement: {first:#?}");
    };
    assert!(
        branch.leading_instead_surface,
        "the authored leading `instead` must survive lowering"
    );
    let [replacement_body] = branch.replacement_effects.as_slice() else {
        panic!("the authored replacement clause must remain one typed body: {branch:#?}");
    };
    let replacement_body = replacement_body
        .downcast_ref::<SequenceEffect>()
        .expect("the replacement clause should retain its authored coordination");
    assert_eq!(
        replacement_body.surface,
        ironsmith_core::SequenceSurface::Coordinated
    );
    let replacement_effects = &replacement_body.effects;
    assert_eq!(replacement_effects.len(), 4, "{replacement_body:#?}");
    assert!(
        replacement_effects[0]
            .downcast_ref::<ShuffleGraveyardIntoLibraryEffect>()
            .is_some()
    );
    let replacement_draw = replacement_effects[1]
        .downcast_ref::<DrawCardsEffect>()
        .expect("the second replacement action should draw X");
    assert_eq!(replacement_draw.count, Value::X);

    let untap = unwrap_surface_wrappers(&replacement_effects[2])
        .downcast_ref::<UntapEffect>()
        .expect("the third replacement action should untap lands");
    let ChooseSpec::WithCount(target, count) = &untap.target else {
        panic!("untap must retain its up-to-five choice: {untap:#?}");
    };
    assert_eq!(count.min, 0);
    assert_eq!(count.max, Some(5));
    let ChooseSpec::Object(filter) = target.as_ref() else {
        panic!("untap choice should be an object filter: {target:#?}");
    };
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.card_types, [CardType::Land]);

    let lasting_rule = replacement_effects[3]
        .downcast_ref::<CantEffect>()
        .expect("the final replacement action should establish the lasting player rule");
    assert_eq!(
        lasting_rule.restriction,
        Restriction::NoMaximumHandSize(PlayerFilter::You)
    );
    assert_eq!(lasting_rule.duration, Until::Forever);
}
