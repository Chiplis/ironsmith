use super::*;
use crate::lexer::{lex_line, render_token_slice};

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("static type-and-color fixture should lex")
}

#[test]
fn typed_static_grant_migration_parses_type_and_color_facts() {
    let skip_tokens = lex("Skip your upkeep step if you control no creatures.");
    let skip = parse_skip_your_upkeep_tokens(&skip_tokens).unwrap();
    let SkipYourUpkeepTail::Condition(condition) = skip.tail else {
        panic!("expected a typed skip-upkeep condition")
    };
    assert_eq!(render_token_slice(condition), "you control no creatures");

    let addition_tokens =
        lex("Lands you control are the chosen type in addition to their other types.");
    let addition = parse_subject_type_addition_tokens(&addition_tokens).unwrap();
    assert!(addition.chosen_type);
    assert_eq!(
        render_token_slice(addition.subject_tokens),
        "Lands you control"
    );

    let creature_type_addition_tokens =
        lex("Creatures you control are Slivers in addition to their other creature types.");
    let creature_type_addition =
        parse_subject_type_addition_tokens(&creature_type_addition_tokens).unwrap();
    assert_eq!(
        render_token_slice(creature_type_addition.subject_tokens),
        "Creatures you control"
    );
    assert_eq!(
        render_token_slice(creature_type_addition.descriptor_tokens),
        "Slivers"
    );

    let identity_tokens = lex("This Vehicle is an artifact creature.");
    let identity = parse_subject_card_type_identity_tokens(&identity_tokens).unwrap();
    assert_eq!(render_token_slice(identity.subject_tokens), "This Vehicle");
    assert_eq!(
        render_token_slice(identity.descriptor_tokens),
        "an artifact creature"
    );

    let animation_tokens = lex("Lands you control are 3/3 creatures that are still lands.");
    let animation = parse_land_animation_tokens(&animation_tokens).unwrap();
    assert_eq!((animation.power, animation.toughness), (3, 3));

    let base_tokens = lex("base power and toughness 4/4, flying and vigilance");
    let base = parse_base_power_toughness_grant_tokens(&base_tokens).unwrap();
    assert_eq!((base.power, base.toughness), (4, 4));
    assert_eq!(
        render_token_slice(base.ability_tokens),
        "flying and vigilance"
    );

    let color_tokens = lex("All creatures are all colors.");
    let color = parse_subject_color_tokens(&color_tokens).unwrap();
    assert_eq!(color.color, Color::ALL.into_iter().collect::<ColorSet>());
    assert!(
        parse_subject_color_tokens(&lex("Enchanted creature gets +3/+1 and is black.")).is_none(),
        "an atomic color fact must not consume a preceding compound predicate"
    );
    let conjoined_subject_tokens = lex("Artifacts and creatures are blue.");
    let conjoined_subject = parse_subject_color_tokens(&conjoined_subject_tokens)
        .expect("a complete conjoined nominal subject should remain valid");
    assert_eq!(
        render_token_slice(conjoined_subject.subject_tokens),
        "Artifacts and creatures"
    );

    let subtype_tokens = lex("Nonbasic lands are Mountains.");
    let subtype = parse_basic_land_subtype_tokens(&subtype_tokens).unwrap();
    assert_eq!(subtype.subtype, Subtype::Mountain);

    let pt_addition_tokens =
        lex("All lands are 2/2 blue creatures in addition to their other types.");
    let pt_addition = parse_power_toughness_type_addition_tokens(&pt_addition_tokens).unwrap();
    assert_eq!((pt_addition.power, pt_addition.toughness), (2, 2));
    assert_eq!(
        render_token_slice(pt_addition.descriptor_tokens),
        "blue creatures"
    );

    let color_addition_tokens =
        lex("All creatures are blue and are Frogs in addition to their other creature types.");
    let color_addition = parse_color_type_addition_tokens(&color_addition_tokens).unwrap();
    assert_eq!(
        render_token_slice(color_addition.descriptor_tokens),
        "Frogs"
    );

    assert!(
            parse_all_cards_chosen_color_addition_tokens(&lex(
                "All cards that aren't on the battlefield, spells, and permanents are the chosen color in addition to their other colors."
            ))
            .is_some()
        );

    assert!(matches!(
        parse_land_type_addition_tokens(&lex(
            "Lands you control are every basic land type in addition to their other types."
        )),
        Some(LandTypeAdditionFact::EveryBasic { .. })
    ));
}
