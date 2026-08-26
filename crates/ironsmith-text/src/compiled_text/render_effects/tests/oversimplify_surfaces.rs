use super::*;

const OVERSIMPLIFY: &str = "Exile all creatures. Each player creates a 0/0 green and blue Fractal creature token and puts a number of +1/+1 counters on it equal to the total power of creatures they controlled that were exiled this way.";

fn compile(text: &str) -> crate::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Oversimplify Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(text)
        .unwrap_or_else(|error| panic!("Oversimplify structure should compile: {error}"))
}

#[test]
fn per_player_created_token_counter_followup_keeps_the_player_partition() {
    let definition = compile(OVERSIMPLIFY);
    let debug = format!("{definition:#?}");

    assert!(debug.contains("ForPlayersEffect"), "{debug}");
    assert!(
        debug.contains("player: Some(") && debug.contains("IteratedPlayer"),
        "{debug}"
    );
    let program = definition.spell_effect.as_ref().expect("spell program");
    let [exile_segment, player_segment] = program.segments.as_slice() else {
        panic!("expected two source-sentence segments: {program:#?}");
    };
    assert!(!exile_segment.starts_new_source_line);
    assert!(player_segment.starts_new_source_line);
    let [exile_root] = exile_segment.default_effects.as_slice() else {
        panic!("expected one exile root: {exile_segment:#?}");
    };
    let exile_id = exiled_all_creatures_effect_id(exile_root)
        .expect("the first segment should be the exact all-creatures producer");
    let [player_root] = player_segment.default_effects.as_slice() else {
        panic!("expected one participant root: {player_segment:#?}");
    };
    let players = structural_unwrap_render_wrappers(player_root)
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("the second segment should retain its player loop");
    let [create_root, counters_root] = players.effects.as_slice() else {
        panic!("expected create/counters player body: {players:#?}");
    };
    let (created_tag, _) = tagged_create_token_effect(create_root).expect("tagged token producer");
    let put = unwrap_structural_effect_tag(counters_root)
        .downcast_ref::<crate::effects::PutCountersEffect>()
        .expect("typed counter follow-up");
    assert!(matches!(&put.target, ChooseSpec::Tagged(tag) if tag == created_tag));
    assert!(
        value_is_iterated_players_total_power_of_effect_affected_creatures(&put.amount, exile_id,),
        "the amount must use the participant's producer partition: {:#?}",
        put.amount
    );
    assert_eq!(
        describe_exile_all_creatures_each_player_fractal_power_counters(&[
            exile_root.clone(),
            player_root.clone(),
        ])
        .as_deref(),
        Some(OVERSIMPLIFY.trim_end_matches('.'))
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        OVERSIMPLIFY
    );
}

#[test]
fn unpartitioned_prior_objects_do_not_gain_they_controlled_wording() {
    let text = "Exile all creatures. Each player creates a 0/0 green and blue Fractal creature token and puts a number of +1/+1 counters on it equal to the total power of creatures that were exiled this way.";
    let rendered = crate::compiled_text::compiled_text_lines(&compile(text)).join("\n");

    assert!(
        !rendered.contains("they controlled"),
        "an unpartitioned metric must not gain participant ownership: {rendered}"
    );
}
