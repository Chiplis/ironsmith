use super::*;

#[test]
fn opposite_coin_branch_preserves_player_or_planeswalker_target_noun() {
    for (player, referenced_player) in [
        (PlayerFilter::Opponent, "opponent"),
        (PlayerFilter::Any, "player"),
    ] {
        let win_amount = Value::Count(ObjectFilter::creature().you_control())
            .with_surface_hint(ValueSurfaceHint::EqualTo);
        let win_damage =
            Effect::deal_damage(win_amount, ChooseSpec::PlayerOrPlaneswalker(player.clone()));

        let lose_count =
            ObjectFilter::creature().controlled_by(PlayerFilter::TargetPlayerOrControllerOfTarget);
        let lose_amount = Value::Count(lose_count).with_surface_hint(ValueSurfaceHint::EqualTo);
        let lose_damage = Effect::deal_damage(lose_amount, ChooseSpec::SourceController);

        let rendered = describe_effect_list(&[
            Effect::with_id(7, Effect::flip_coin(PlayerFilter::You)),
            Effect::if_then_else(
                crate::effect::EffectId(7),
                EffectPredicate::Happened,
                vec![win_damage],
                vec![],
            ),
            Effect::if_then_else(
                crate::effect::EffectId(7),
                EffectPredicate::DidNotHappen,
                vec![lose_damage],
                vec![],
            ),
        ]);

        assert_eq!(
            rendered,
            format!(
                "Flip a coin. If you win the flip, deal damage to target {referenced_player} or planeswalker equal to the number of creatures you control. If you lose the flip, deal damage to you equal to the number of creatures that {referenced_player} or that planeswalker's controller controls"
            )
        );
    }
}

#[test]
fn quoted_token_creation_then_coin_flip_preserves_all_public_instructions() {
    let text = "At the beginning of your upkeep, create a colorless artifact token named Land Mine with \"{R}, Sacrifice this token: This token deals 2 damage to target attacking creature without flying.\" Then flip a coin. If you lose the flip, this creature deals 2 damage to itself.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Goblin Kaboomist")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![
                crate::types::Subtype::Goblin,
                crate::types::Subtype::Warrior,
            ])
            .parse_text(text)
            .expect("quoted token creation and coin-flip trigger should compile");
    let debug = format!("{definition:#?}");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text,
        "{debug}"
    );
}
