#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn sparkspitter_renders_the_exact_post_create_token_definition() {
    let oracle = "{R}, {T}, Discard a card: Create a 3/1 red Elemental creature token named Spark Elemental. It has trample, haste, and \"At the beginning of the end step, sacrifice this token.\"";
    let definition = parse_oracle_card_definition("Sparkspitter");

    assert_eq!(canonical_compiled_lines(&definition).join("\n"), oracle);

    let create = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .iter()
                .find_map(|effect| effect.downcast_ref::<CreateTokenEffect>()),
            _ => None,
        })
        .expect("Sparkspitter must create its typed Spark Elemental token");
    let keyword_ids = create
        .token
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(create.token.card.name, "Spark Elemental");
    assert_eq!(
        keyword_ids,
        [StaticAbilityId::Trample, StaticAbilityId::Haste,]
    );
    assert_eq!(
        create.ability_presentation,
        Some(ironsmith_core::TokenAbilityPresentation::SeparateSentenceCombined)
    );
    assert!(create.sacrifice_at_next_end_step);
    assert_eq!(create.next_end_step_player, PlayerFilter::Any);
}
