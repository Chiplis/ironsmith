#![allow(unused_imports)]
use super::shard_00::*;
use super::shard_01::*;
use super::shard_02::*;
use super::shard_03::*;
use super::shard_04::*;
use super::shard_05::*;
use super::shard_06::*;
use super::shard_07::*;
use super::shard_08::*;
use super::shard_09::*;
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

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_terrapact_intimidator_preserves_have_you_create_branch() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Terrapact Intimidator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Kavu, Subtype::Scout])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "When this creature enters, target opponent may have you create two Lander tokens. If they don't, put two +1/+1 counters on this creature.",
        )
        .expect("Terrapact Intimidator text should parse");

    assert_eq!(def.abilities.len(), 1);
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target opponent may have you create two lander tokens"),
        "expected have-you create wording, got {joined}"
    );
    assert!(
        joined.contains("if they don't, put two +1/+1 counters on this creature"),
        "expected pronoun-based decline branch, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_covenant_of_minds_preserves_opponent_choice_and_decline_branch() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(416_862), "Covenant of Minds")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal the top three cards of your library. Target opponent may choose to put those cards into your hand. If they don't, put those cards into your graveyard and draw five cards.",
        )
        .expect("Covenant of Minds oracle text should parse strictly");

    let expected = concat!(
        "Reveal the top three cards of your library. ",
        "Target opponent may choose to put those cards into your hand. ",
        "If they don't, put those cards into your graveyard and draw five cards."
    );
    assert_eq!(
        unprocessed_compiled_lines(&def),
        vec![expected.to_string()],
        "expected exact Covenant of Minds compiled text"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("LookAtTopCardsEffect")
            && debug.contains("MayEffect")
            && debug.contains("IfEffect"),
        "expected reveal, optional opponent choice, and decline conditional effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_target_opponent_may_draw_card() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Target Opponent May Draw Variant")
        .parse_text("At the beginning of your end step, target opponent may draw a card.")
        .expect("target-opponent may-draw trigger should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target opponent may") && joined.contains("draw"),
        "expected target-opponent may-draw text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_it_connives_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Connive It Variant")
        .parse_text("When this creature enters, it connives.")
        .expect("it-connives trigger clause should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("connive"),
        "expected connive text to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_connive_x_where_clause_preserves_dynamic_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Connive X Variant")
        .parse_text(
            "Whenever you attack, target attacking creature connives X, where X is the number of attacking creatures.",
        )
        .expect("connive X where-clause should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    let debug = format!("{:?}", def.abilities);
    assert!(
        joined.contains("connives") && joined.contains("number of attacking creatures"),
        "expected dynamic connive text to be preserved, got {joined}"
    );
    assert!(
        debug.contains("ConniveEffect") && debug.contains("Count"),
        "expected ConniveEffect with dynamic count, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_hand_choose_card_from_it_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reveal Hand From It Variant")
        .parse_text(
            "Target opponent reveals their hand. You choose a nonland card from it and exile that card.",
        )
        .expect("reveal-hand choose-from-it chain should parse");
    let joined = format!("{:#?}", def.spell_effect).to_lowercase();
    assert!(
        joined.contains("lookathandeffect")
            && joined.contains("chooseobjectseffect")
            && (joined.contains("exileeffect") || joined.contains("movetozoneeffect")),
        "expected reveal-hand choose-then-exile effect chain, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_partial_reveal_from_hand_choose_one_of_them_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Blackmail Variant")
        .parse_text(
            "Target player reveals three cards from their hand and you choose one of them. That player discards that card.",
        )
        .expect("partial hand reveal then choose-one-of-them chain should parse");
    let joined = format!("{:#?}", def.spell_effect).to_lowercase();
    let compact = joined.split_whitespace().collect::<String>();
    assert!(
        joined.contains("chooseobjectseffect")
            && joined.contains("reveal: true")
            && compact.contains("zone:some(hand")
            && joined.contains("discard"),
        "expected partial hand reveal, chosen-card link, and discard effect chain, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_dynamic_partial_reveal_from_hand_keeps_count_and_player_links() {
    for (text, expected_fragments) in [
        (
            "Target player reveals X cards from their hand and you choose one of them. That player discards that card.",
            &["target player reveals x cards from their hand and you choose one of them"][..],
        ),
        (
            "Target player reveals a number of cards from their hand equal to one plus the number of creature cards in your graveyard. You choose one of them. That player discards that card.",
            &[
                "target player reveals a number of cards from their hand equal to",
                "creature cards in your graveyard",
                "you choose one of them",
                "that player discards that card",
            ][..],
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), "Dynamic Partial Reveal Variant")
            .parse_text(text)
            .expect("dynamic partial hand reveal should parse");
        let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
        for expected in expected_fragments {
            assert!(
                joined.contains(expected),
                "expected {expected:?} in {joined}"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_target_opponent_gains_control_of_it_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gain Control Of It Variant")
        .parse_text("When this creature enters, target opponent gains control of it.")
        .expect("gain-control-of-it trigger clause should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("target opponent gains control"),
        "expected gain-control text to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_trigger_destroy_it_then_cant_regenerate_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Destroy It No Regen Variant")
        .parse_text(
            "Whenever this creature deals combat damage to a creature, destroy it. It can't be regenerated.",
        )
        .expect("destroy-it then cant-regenerate trigger should parse");
    let joined = unprocessed_compiled_lines(&def).join(" ").to_lowercase();
    assert!(
        joined.contains("destroy") && joined.contains("can't be regenerated"),
        "expected destroy/no-regeneration sequence to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_player_multi_step_then_clause_fails_instead_of_partial_parse() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Each Player Multi-step Variant")
            .parse_text(
                "Each player loses X life, discards X cards, sacrifices X creatures of their choice, then sacrifices X lands of their choice.",
            )
            .expect_err("unsupported each-player then-clause should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported multi-step each-player clause with 'then'")
            || message.contains("unsupported each-player lose/discard/sacrifice chain clause"),
        "expected strict each-player multi-step parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_transformed_clause_uses_shared_return_and_transform() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Transformed Return Variant")
        .parse_text(
            "When this creature dies, return it to the battlefield transformed under your control.",
        )
        .expect("transformed return should parse through shared return path");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("return it to the battlefield transformed under your control"),
        "expected structural return-transformed rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_transformed_clause_raw_render_compacts_structural_return_and_transform()
{
    let def = CardDefinitionBuilder::new(CardId::new(), "Harvest Hand Variant")
        .parse_text(
            "When this creature dies, return it to the battlefield transformed under your control.",
        )
        .expect("transformed return should parse through shared return path");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "When this creature dies, return it to the battlefield transformed under your control."
        ),
        "expected raw unprocessed_compiled_lines output to compact proven move-then-transform semantics, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_transformed_clause_canonical_output_matches_oracle_style_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Harvest Hand Canonical Variant")
        .parse_text(
            "When this creature dies, return it to the battlefield transformed under your control.",
        )
        .expect("transformed return should canonicalize to oracle-style wording");
    let rendered = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
    assert_eq!(
        rendered,
        "When this creature dies, return it to the battlefield transformed under your control."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_converted_clause_uses_shared_return_and_convert() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Converted Return Variant")
        .parse_text(
            "When this creature dies, return it to the battlefield converted under your control.",
        )
        .expect("converted return should parse through shared return path");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("return")
            || rendered.contains("put that card onto the battlefield under your control")
            || rendered.contains("put it onto the battlefield under your control"))
            && rendered.contains("convert")
            && !rendered.contains("transform"),
        "expected shared return plus convert lowering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_next_upkeep_clause_fails_instead_of_immediate_return() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Next Upkeep Return Variant")
            .parse_text(
                "When this creature dies, return it to the battlefield tapped under its owner's control at the beginning of their next upkeep.",
            )
            .expect_err("unsupported delayed return timing should fail parse");
    let message = format!("{err:?}");
    assert!(
        message.contains("unsupported delayed return timing clause")
            || message.contains("unsupported triggered line"),
        "expected strict delayed-return parse error, got {message}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_exile_name_and_target_supports_exiling_source_and_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mangara Variant")
        .parse_text("{T}: Exile Mangara of Corondor and target permanent.")
        .expect("named-source + target exile should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (rendered.contains("exile this creature") || rendered.contains("exile this permanent"))
            && rendered.contains("target permanent"),
        "expected exile of source and target permanent, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_chaotic_transformation_reuses_single_exiled_helper_tag() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Chaotic Transformation Variant")
        .parse_text(
            "Exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land. For each permanent exiled this way, its controller reveals cards from the top of their library until they reveal a card that shares a card type with it, puts that card onto the battlefield, then shuffles.",
        )
        .expect("Chaotic Transformation pattern should parse");
    let spell_debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    let needle = "__sentence_helper_exiled_";
    let mut tags = std::collections::BTreeSet::new();
    let mut rest = spell_debug.as_str();
    while let Some(idx) = rest.find(needle) {
        let after = &rest[idx..];
        let end = after
            .find('\'')
            .or_else(|| after.find(' '))
            .unwrap_or(after.len());
        tags.insert(after[..end].to_string());
        rest = &after[needle.len()..];
    }

    assert!(
        !tags.is_empty(),
        "expected Chaotic Transformation lowering to use an exiled helper tag, got {spell_debug}"
    );
    assert_eq!(
        tags.len(),
        1,
        "expected a single shared exiled helper tag throughout Chaotic Transformation compile, got {tags:?} in {rendered}"
    );
    assert_eq!(
        spell_debug.matches("ChooseObjectsEffect").count(),
        5,
        "expected five independently optional targets, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ConsultTopOfLibraryEffect"),
        "expected reveal-until consult lowering, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("SearchLibraryEffect"),
        "expected consult lowering instead of generic search, got {spell_debug}"
    );
    let normalized = rendered.to_ascii_lowercase();
    assert!(
        normalized.contains(
            "exile up to one target artifact, up to one target creature, up to one target enchantment, up to one target planeswalker, and/or up to one target land"
        )
            && normalized.contains("for each permanent exiled this way")
            && normalized.contains("shares a card type with it")
            && normalized.contains("puts that card onto the battlefield, then shuffles"),
        "expected canonical optional-target exile and consult rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_repeated_optional_exile_targets_return_as_one_plural_collection() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Independent Exile Return Variant")
        .parse_text(
            "At the beginning of your end step, exile up to one target artifact you control and up to one target creature you control. Then return them to the battlefield under their owners' control.",
        )
        .expect("independent optional exile targets should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert_eq!(
        debug.matches("ChooseObjectsEffect").count(),
        2,
        "expected two independently constrained target choices, got {debug}"
    );
    assert!(
        debug.contains("ReturnAllToBattlefieldEffect")
            && !debug.contains(crate::tag::SOURCE_EXILED_TAG),
        "expected preparation to bind the plural return to the aggregate helper-exile tag, got {debug}"
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("return the exiled cards")
            && rendered
                .to_ascii_lowercase()
                .contains("under their owners' control"),
        "expected plural tagged return, got {rendered}; debug={debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_relative_counter_condition_keeps_spell_controller_comparison() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Relative Counter Variant")
        .parse_text(
            "Counter target spell if you control more creatures than that spell's controller.",
        )
        .expect("relative spell-controller condition should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");

    assert!(
        debug.contains("YouControlMoreCreaturesThanTargetSpellController"),
        "expected typed relative predicate, got {debug}"
    );
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("more creatures than the target spell's controller"),
        "expected relative controller comparison in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_attached_count_conjunction_and_related_subjects_without_loss() {
    let suit = CardDefinitionBuilder::new(CardId::new(), "Attached Count Variant")
        .parse_text(
            "Equipped creature gets +1/+1 for each Aura and Equipment attached to it and has ward {2}.\nEquip {2}",
        )
        .expect("compound attached count should parse");
    let suit_debug = format!("{suit:#?}");
    let suit_rendered = unprocessed_compiled_lines(&suit).join(" ");
    assert!(
        suit_debug.contains("Aura")
            && suit_debug.contains("Equipment")
            && suit_debug.contains("Ward"),
        "expected both attached permanent kinds and ward, got {suit_debug}"
    );
    assert!(
        suit_rendered.to_ascii_lowercase().contains("aura")
            && suit_rendered.to_ascii_lowercase().contains("equipment")
            && suit_rendered.to_ascii_lowercase().contains("ward {2}"),
        "expected complete attached count surface, got {suit_rendered}"
    );

    let crown = CardDefinitionBuilder::new(CardId::new(), "Attached Related Variant")
        .parse_text(
            "Enchant creature\nEnchanted creature gets +1/+0 and has first strike.\nSacrifice this Aura: Enchanted creature and other creatures that share a creature type with it get +1/+0 and gain first strike until end of turn.",
        )
        .expect("attached object and related creatures should parse");
    let crown_debug = format!("{crown:#?}");
    let crown_rendered = unprocessed_compiled_lines(&crown).join(" ");
    assert!(
        crown_debug.contains("SharesSubtypeWithTagged")
            && crown_debug.contains("IsNotTaggedObject"),
        "expected the related-creature branch to remain structural, got {crown_debug}"
    );
    assert!(
        crown_rendered
            .to_ascii_lowercase()
            .contains("other creatures")
            && crown_rendered
                .to_ascii_lowercase()
                .contains("share a creature type"),
        "expected both activated-ability subjects in compiled text, got {crown_rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_quoted_ward_sacrifice_grant_compacts_as_sacrifice_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quoted Ward Grant Variant")
        .parse_text("Permanents you control have \"Ward—Sacrifice a permanent.\"")
        .expect("quoted ward sacrifice grant should parse");
    let debug = format!("{def:#?}");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        debug.contains("Ward(TotalCost") && debug.contains("SacrificeEffect"),
        "expected a structural sacrifice ward cost, got {debug}"
    );
    assert!(
        rendered_lower.contains("ward—sacrifice a permanent")
            && !rendered_lower.contains("ward—exile"),
        "expected compact sacrifice cost surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_shape_anew_targets_controller_and_consults_until_artifact() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shape Anew")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "The controller of target artifact sacrifices it, then reveals cards from the top of their library until they reveal an artifact card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.",
        )
        .expect("Shape Anew should parse");

    let spell_debug = format!("{:#?}", def.spell_effect);
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        spell_debug.contains("SacrificeTargetEffect"),
        "expected targeted sacrifice lowering, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ConsultTopOfLibraryEffect"),
        "expected reveal-until consult lowering, got {spell_debug}"
    );
    assert!(
        !spell_debug.contains("SearchLibraryEffect"),
        "expected consult lowering instead of generic search, got {spell_debug}"
    );
    assert!(
        spell_debug.contains("ControllerOf"),
        "expected the follow-up to keep controller-of-target binding, got {spell_debug}"
    );
    assert!(
        rendered.contains("target artifact")
            && rendered.contains("until they reveal an artifact card"),
        "expected Shape Anew rendering to preserve target and controller binding, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn shape_anew_sacrifices_target_and_uses_that_controller_library() {
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};

    fn artifact(name: &str) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build()
    }

    fn filler(name: &str) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .build()
    }

    let shape_anew = CardDefinitionBuilder::new(CardId::new(), "Shape Anew")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "The controller of target artifact sacrifices it, then reveals cards from the top of their library until they reveal an artifact card. That player puts that card onto the battlefield, then shuffles all other cards revealed this way into their library.",
        )
        .expect("Shape Anew should parse");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let target_artifact = game.create_object_from_definition(
        &artifact("Bob Target Artifact"),
        bob,
        crate::zone::Zone::Battlefield,
    );
    let bob_library_artifact = game.create_object_from_definition(
        &artifact("Bob Library Artifact"),
        bob,
        crate::zone::Zone::Library,
    );
    game.create_object_from_definition(
        &filler("Bob Library Filler"),
        bob,
        crate::zone::Zone::Library,
    );

    let alice_library_artifact = game.create_object_from_definition(
        &artifact("Alice Library Artifact"),
        alice,
        crate::zone::Zone::Library,
    );
    game.create_object_from_definition(
        &filler("Alice Library Filler"),
        alice,
        crate::zone::Zone::Library,
    );

    let source = game.new_object_id();
    let mut ctx = ExecutionContext::new_default(source, alice)
        .with_targets(vec![ResolvedTarget::Object(target_artifact)]);
    ctx.snapshot_targets(&game);

    for effect in shape_anew.spell_effect.as_ref().expect("spell effects") {
        execute_effect(&mut game, effect, &mut ctx)
            .expect("shape anew effect should resolve cleanly");
    }

    assert!(
        game.player(bob)
            .expect("bob exists")
            .graveyard
            .iter()
            .any(|&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Bob Target Artifact")
            }),
        "target artifact should have been sacrificed"
    );
    assert!(
        game.battlefield.iter().any(|&id| {
            game.object(id).is_some_and(|obj| {
                obj.name == "Bob Library Artifact" && game.controller_of(obj) == bob
            })
        }),
        "Bob's library artifact should enter the battlefield under Bob's control"
    );
    assert!(
        !game.battlefield.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Alice Library Artifact")
        }),
        "Alice's library should not be consulted for Bob's target artifact"
    );
    assert!(
        game.object(bob_library_artifact).is_none(),
        "Bob's artifact should have left the library"
    );
    assert!(
        game.object(alice_library_artifact).is_some(),
        "Alice's library artifact should remain untouched"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_opponent_exiles_card_from_their_hand_uses_hand_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skullcap Snail Variant")
        .parse_text("Target opponent exiles a card from their hand.")
        .expect("parse targeted hand exile");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("TargetOnlyEffect"),
        "expected target-opponent context setup, got {debug}"
    );
    assert!(
        debug.contains("ChooseObjectsEffect"),
        "expected choose-from-hand effect, got {debug}"
    );
    assert!(
        debug.contains("filter: ObjectFilter { zone: Some(Hand)"),
        "expected choose-from-hand filter zone, got {debug}"
    );
    assert!(
        debug.contains("chooser: Target(Opponent)"),
        "expected target opponent chooser, got {debug}"
    );
    assert!(
        debug.contains("ExileEffect")
            && (debug.contains("Tagged(TagKey(\"exiled_0\"))")
                || debug.contains("Tagged(TagKey(\"__sentence_helper_exiled_")),
        "expected exile of chosen tagged card, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_each_opponent_exiles_card_from_their_hand_uses_iterated_chooser() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Each Opponent Hand Exile Variant")
        .parse_text("Each opponent exiles a card from their hand.")
        .expect("parse each-opponent hand exile");

    let effects = def.spell_effect.expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("ForPlayersEffect"),
        "expected foreach-opponent wrapper, got {debug}"
    );
    assert!(
        debug.contains("chooser: IteratedPlayer"),
        "expected iterated chooser for each-opponent hand exile, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_eldrazi_spawn_reminder_sentence_is_not_immediate_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dread Drone Variant")
            .parse_text(
                "When this creature enters, create two 0/1 colorless Eldrazi Spawn creature tokens. They have \"Sacrifice this creature: Add {C}.\"",
            )
            .expect("parse eldrazi spawn reminder");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    assert_eq!(
        triggered.effects.len(),
        1,
        "spawn reminder must not compile as a second immediate effect"
    );

    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Eldrazi Spawn creature token"),
        "expected spawn token in compiled text, got {joined}"
    );
    assert!(
        !joined.contains("sacrifice"),
        "spawn reminder must not add immediate sacrifice clause, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_eldrazi_scion_reminder_sentence_is_not_immediate_sacrifice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scion Variant")
            .parse_text(
                "When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has \"Sacrifice this creature: Add {C}.\"",
            )
            .expect("parse eldrazi scion reminder");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let effects = &triggered.effects;
    assert_eq!(
        effects.len(),
        1,
        "scion reminder must not compile as a second immediate effect"
    );
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("Eldrazi Scion"),
        "expected scion token creation, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spawn_scion_mana_reminder_without_context_fails_strictly() {
    let _err = CardDefinitionBuilder::new(CardId::new(), "Standalone Spawn Reminder")
        .parse_text("They have \"Sacrifice this creature: Add {C}.\"")
        .expect_err("standalone token reminder should fail");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_growth_spasm_style_spawn_reminder_stays_statement_not_static() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Growth Spasm Variant")
            .parse_text(
                "Search your library for a basic land card, put it onto the battlefield tapped, then shuffle. Create a 0/1 colorless Eldrazi Spawn creature token. It has \"Sacrifice this token: Add {C}.\"",
            )
            .expect("growth spasm line should parse as statement");

    assert!(
        def.spell_effect.is_some(),
        "expected spell effects for spell text"
    );
    assert!(
        def.abilities.is_empty(),
        "statement text must not be misclassified as static ability"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_convoked_connive_clause_compiles_to_tagged_connive_iteration() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lethal Scheme Variant")
        .parse_text("Destroy target creature or planeswalker. Each creature that convoked this spell connives.")
        .expect("parse convoked connive clause");

    let effects = def.spell_effect.as_ref().expect("expected spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("convoked_this_spell"),
        "expected convoked tag reference, got {debug}"
    );
    assert!(
        debug.contains("ConniveEffect"),
        "expected connive effect in compiled spell effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_convoked_it_creature_etb_reference_compiles_to_tagged_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Venerated Loxodon Variant")
        .parse_text(
            "Convoke\nWhen this creature enters, put a +1/+1 counter on each creature that convoked it.",
        )
        .expect("parse convoked-it reference");

    let triggered = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("expected triggered ability");
    let debug = format!("{:?}", triggered.effects);
    assert!(
        debug.contains("convoked_this_spell"),
        "expected convoked tag reference in effects, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_mother_of_runes_compacts_protection_choice_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mother of Runes Variant")
            .parse_text(
                "{T}: Target creature you control gains protection from the color of your choice until end of turn.",
            )
            .expect("mother of runes line should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("gains protection from the color of your choice until end of turn"),
        "expected compact protection-choice rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_giver_of_runes_compacts_colorless_or_color_choice_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Giver of Runes Variant")
            .parse_text(
                "{T}: Another target creature you control gains protection from colorless or from the color of your choice until end of turn.",
            )
            .expect("giver of runes line should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains(
            "gains protection from colorless or from the color of your choice until end of turn"
        ),
        "expected compact colorless-or-color protection rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_root_greevil_compacts_destroy_color_choice_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Root Greevil Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Beast])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "{2}{G}, {T}, Sacrifice this creature: Destroy all enchantments of the color of your choice.",
        )
        .expect("root greevil text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChooseModeEffect") && debug.matches("DestroyEffect").count() == 5,
        "expected five-color modal lowering, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Destroy all enchantments of the color of your choice"),
        "expected compact color-choice rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_wash_out_compacts_return_color_choice_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wash Out Variant")
        .parse_text("Return all permanents of the color of your choice to their owners' hands.")
        .expect("wash out text should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseModeEffect") && debug.matches("ReturnToHandEffect").count() == 5,
        "expected five-color modal return lowering, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .contains("Return all permanents of the color of your choice to their owners' hands"),
        "expected compact return color-choice rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_draw_for_each_creature_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Collective Unconscious Variant")
        .parse_text("Draw a card for each creature you control.")
        .expect("draw-for-each should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("draw a card for each creature you control"),
        "expected oracle-like draw-for-each wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_draw_for_each_subtype_uses_oracle_like_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sea Gate Loremaster Variant")
        .parse_text("{T}: Draw a card for each Ally you control.")
        .expect("subtype draw-for-each should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Draw a card for each Ally you control"),
        "expected subtype draw-for-each wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_flow_of_knowledge_preserves_for_each_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flow of Knowledge")
        .card_types(vec![CardType::Instant])
        .parse_text("Draw a card for each Island you control, then discard two cards.")
        .expect("Flow of Knowledge should parse");
    let rendered = compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("Draw a card for each Island you control, then discard two cards"),
        "expected the for-each draw surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_mind_sludge_preserves_for_each_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Mind Sludge")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target player discards a card for each Swamp you control.")
        .expect("Mind Sludge should parse");
    let rendered = compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("Target player discards a card for each Swamp you control"),
        "expected the for-each discard surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_huatli_radiant_champion_preserves_for_each_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Huatli, Radiant Champion")
        .card_types(vec![CardType::Planeswalker])
        .loyalty(3)
        .parse_text("+1: Put a loyalty counter on this for each creature you control.")
        .expect("Huatli's first loyalty ability should parse");
    let rendered = compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains("Put a loyalty counter on this for each creature you control"),
        "expected the for-each counter surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_bioessence_hydra_preserves_for_each_counter_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bioessence Hydra")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "This creature enters with a +1/+1 counter on it for each loyalty counter on planeswalkers you control.",
        )
        .expect("Bioessence Hydra's enters ability should parse");
    let rendered = compiled_text_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "This creature enters with a +1/+1 counter on it for each loyalty counter on planeswalkers you control"
        ),
        "expected the for-each enters-with-counters surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_treasure_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Glittermonger Variant")
        .parse_text("{T}: Create a Treasure token.")
        .expect("treasure token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Create a Treasure token"),
        "expected compact treasure token wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_map_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spyglass Siren Variant")
        .parse_text("When this creature enters, create a Map token.")
        .expect("map token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("create a Map token") || joined.contains("Create a Map token"),
        "expected compact map token wording, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "map token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_lander_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Galactic Wayfarer Variant")
        .parse_text("When this creature enters, create a Lander token.")
        .expect("lander token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("create a Lander token") || joined.contains("Create a Lander token"),
        "expected compact lander token wording, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "lander token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_junk_token_uses_expected_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Junk Maker Variant")
        .parse_text("When this creature enters, create a Junk token.")
        .expect("junk token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("create a Junk token") || joined.contains("Create a Junk token"),
        "expected junk token rendering, got {joined}"
    );
    assert!(
        joined
            .to_ascii_lowercase()
            .contains("activate only as a sorcery"),
        "expected junk token rules text to include sorcery restriction, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "junk token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_create_supported_role_tokens_attached_to_creature() {
    let role_names = [
        "Young Hero Role",
        "Monster Role",
        "Sorcerer Role",
        "Royal Role",
        "Cursed Role",
    ];

    for role_name in role_names {
        let text = format!("Create a {role_name} token attached to target creature you control.");
        let def = CardDefinitionBuilder::new(CardId::new(), format!("{role_name} Variant"))
            .parse_text(&text)
            .unwrap_or_else(|err| panic!("{role_name} token creation should parse: {err:?}"));
        let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
        assert!(
            joined
                .to_ascii_lowercase()
                .contains(&role_name.to_ascii_lowercase()),
            "expected compiled text to include role token name '{role_name}', got {joined}"
        );
        assert!(
            !joined
                .to_ascii_lowercase()
                .contains("unsupported parser line fallback"),
            "{role_name} token creation should not rely on unsupported fallback marker, got {joined}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_attached_role_reflexive_fight_uses_enchanted_creature_not_role_token() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Attached Role Fight")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Create a Monster Role token attached to target creature you control. \
             When you do, that creature fights up to one target creature you don't control.",
        )
        .expect("attached Role fight spell should parse");

    let spell_effect = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{spell_effect:?}");
    assert!(
        debug.contains("AttachObjectsEffect") && debug.contains("attachment_target_1"),
        "expected Role attachment target to be tagged for the follow-up fight, got {debug}"
    );
    assert!(
        debug.contains("FightEffect { creature1: Tagged(TagKey(\"attachment_target_1\"))"),
        "expected the enchanted creature, not the Role token, to fight, got {debug}"
    );
    assert!(
        !debug.contains("FightEffect { creature1: Tagged(TagKey(\"created_0\"))"),
        "the created Aura Role token must not be used as the fighting creature: {debug}"
    );
    assert!(
        debug.contains("Anthem") && debug.contains("GrantAbility") && debug.contains("Trample"),
        "expected Monster Role token to carry its enchanted-creature buff and trample grant, got {debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Create a Monster Role token attached to target creature you control. When you do, that creature fights up to one target creature you don't control."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_draw_and_life_loss_with_shared_dynamic_x() {
    let plain_x_def = CardDefinitionBuilder::new(CardId::new(), "Plain X Draw Loss")
        .card_types(vec![CardType::Sorcery])
        .parse_text("You draw X cards and you lose X life.")
        .expect("plain X draw/loss spell should parse");
    assert!(matches!(
        unprocessed_compiled_lines(&plain_x_def).join("\n").as_str(),
        "You draw X cards and you lose X life." | "Draw X cards and you lose X life."
    ));

    let count_def = CardDefinitionBuilder::new(CardId::new(), "Shared Count Draw Loss")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You draw X cards and you lose X life, where X is the number of creatures you control.",
        )
        .expect("shared count draw/loss spell should parse");
    assert!(matches!(
        unprocessed_compiled_lines(&count_def).join("\n").as_str(),
        "You draw X cards and you lose X life, where X is the number of creatures you control."
            | "Draw X cards and you lose X life, where X is the number of creatures you control."
    ));

    let devotion_def = CardDefinitionBuilder::new(CardId::new(), "Shared Devotion Draw Loss")
        .card_types(vec![CardType::Sorcery])
        .parse_text("You draw X cards and you lose X life, where X is your devotion to black.")
        .expect("shared devotion draw/loss spell should parse");
    assert!(matches!(
        unprocessed_compiled_lines(&devotion_def)
            .join("\n")
            .as_str(),
        "You draw X cards and you lose X life, where X is your devotion to black."
            | "Draw X cards and you lose X life, where X is your devotion to black."
    ));

    let target_devotion_def = CardDefinitionBuilder::new(CardId::new(), "Target Devotion Draw")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent draws X cards where X is their devotion to black.")
        .expect("target opponent devotion draw spell should parse");
    let target_debug = format!("{:?}", target_devotion_def.spell_effect);
    assert!(
        target_debug.contains("DrawCardsEffect")
            && target_debug.contains("target: Target(Player(Opponent))")
            && target_debug.contains("Devotion { player: Target(Opponent), color: Black }")
            && target_debug.contains("color: Black"),
        "expected their devotion to bind to target opponent, got {target_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn player_scoped_their_devotion_binds_across_common_effect_amounts() {
    fn assert_target_opponent_devotion(name: &str, text: &str, effect_name: &str) {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));
        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains(effect_name)
                && debug.contains("target: Target(Player(Opponent))")
                && debug.contains("Devotion { player: Target(Opponent), color: Black }"),
            "expected their devotion to bind to target opponent for {name}, got {debug}"
        );
    }

    assert_target_opponent_devotion(
        "Their Devotion Draw",
        "Target opponent draws X cards where X is their devotion to black.",
        "DrawCardsEffect",
    );
    assert_target_opponent_devotion(
        "Their Devotion Lose Life",
        "Target opponent loses X life, where X is their devotion to black.",
        "LoseLifeEffect",
    );
    assert_target_opponent_devotion(
        "Their Devotion Gain Life",
        "Target opponent gains X life, where X is their devotion to black.",
        "GainLifeEffect",
    );
    assert_target_opponent_devotion(
        "Their Devotion Mill",
        "Target opponent mills X cards, where X is their devotion to black.",
        "MillEffect",
    );
    assert_target_opponent_devotion(
        "Their Devotion Poison Counters",
        "Target opponent gets X poison counters, where X is their devotion to black.",
        "PoisonCountersEffect",
    );

    for (name, text, effect_name) in [
        (
            "Each Opponent Their Devotion Scry",
            "Each opponent scries X, where X is their devotion to black.",
            "ScryEffect",
        ),
        (
            "Each Opponent Their Devotion Surveil",
            "Each opponent surveils X, where X is their devotion to black.",
            "SurveilEffect",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));
        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains("ForPlayersEffect")
                && debug.contains(effect_name)
                && debug.contains("Devotion { player: IteratedPlayer, color: Black }"),
            "expected their devotion to bind to the iterated opponent for {name}, got {debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn player_scoped_their_filters_bind_across_common_object_effects() {
    let discard_one = CardDefinitionBuilder::new(CardId::new(), "Their Hand Discard One")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent discards a card from their hand.")
        .expect("target opponent discard from their hand should parse");
    let discard_one_debug = format!("{:?}", discard_one.spell_effect);
    assert!(
        discard_one_debug.contains("DiscardEffect")
            && !discard_one_debug.contains("DiscardHandEffect")
            && discard_one_debug.contains("player: Target(Opponent)")
            && discard_one_debug.contains("owner: Some(Target(Opponent))"),
        "expected their hand to bind to target opponent for single discard, got {discard_one_debug}"
    );

    let discard_all = CardDefinitionBuilder::new(CardId::new(), "Their Hand Discard All")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent discards all cards from their hand.")
        .expect("target opponent discard all from their hand should parse");
    let discard_all_debug = format!("{:?}", discard_all.spell_effect);
    assert!(
        discard_all_debug.contains("DiscardEffect")
            && discard_all_debug.contains("count: Count")
            && discard_all_debug.contains("player: Target(Opponent)")
            && discard_all_debug.contains("owner: Some(Target(Opponent))"),
        "expected all cards from their hand to bind to target opponent, got {discard_all_debug}"
    );

    let sacrifice = CardDefinitionBuilder::new(CardId::new(), "Their Creature Sacrifice")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent sacrifices a creature they control.")
        .expect("target opponent sacrifice creature they control should parse");
    let sacrifice_debug = format!("{:?}", sacrifice.spell_effect);
    assert!(
        sacrifice_debug.contains("ChooseObjectsEffect")
            && sacrifice_debug.contains("controller: Some(Target(Opponent))")
            && sacrifice_debug.contains("SacrificePlayerEffect")
            && sacrifice_debug.contains("player: Target(Opponent)"),
        "expected they control to bind to target opponent for sacrifice, got {sacrifice_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn player_subject_context_binds_remaining_simple_player_effects() {
    for (name, text, effect_name) in [
        (
            "Target Opponent Monarch",
            "Target opponent becomes the monarch.",
            "BecomeMonarchEffect",
        ),
        (
            "Target Opponent Skip Turn",
            "Target opponent skips their next turn.",
            "SkipTurnEffect",
        ),
        (
            "Target Opponent Shuffle Graveyard",
            "Target opponent shuffles their graveyard into their library.",
            "ShuffleGraveyardIntoLibraryEffect",
        ),
        (
            "Target Opponent Adds Mana",
            "Target opponent adds {B}.",
            "AddManaEffect",
        ),
        (
            "Target Opponent Loses Game",
            "Target opponent loses the game.",
            "LoseTheGameEffect",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));
        let debug = format!("{:?}", def.spell_effect);
        assert!(
            debug.contains(effect_name)
                && debug.contains("TargetOnlyEffect")
                && debug.contains("target: Target(Player(Opponent))")
                && debug.contains("player: Target(Opponent)"),
            "expected simple player effect to bind target opponent for {name}, got {debug}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn player_subject_roles_keep_chooser_owner_and_affected_player_distinct() {
    let search_def = CardDefinitionBuilder::new(CardId::new(), "Thada Role Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature deals combat damage to a player, search that player's library for an artifact card and exile it. Then that player shuffles. Until end of turn, you may play that card.",
        )
        .expect("combat-damage library-owner search should parse");
    let search_debug = format!("{:#?}", search_def.abilities);
    let compact_search_debug = search_debug.split_whitespace().collect::<String>();
    assert!(
        compact_search_debug.contains("ChooseObjectsEffect")
            && compact_search_debug.contains("owner:Some(DamagedPlayer")
            && compact_search_debug.contains("chooser:You")
            && compact_search_debug.contains("ShuffleLibraryEffect")
            && compact_search_debug.contains("player:DamagedPlayer")
            && compact_search_debug.contains("GrantPlayTaggedEffect")
            && compact_search_debug.contains("player:You"),
        "expected chooser, library owner, shuffle player, and play-grant player to stay distinct, got {search_debug}"
    );

    let coercion_def = CardDefinitionBuilder::new(CardId::new(), "Coercion Role Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent reveals their hand. You choose a card from it. That player discards that card.",
        )
        .expect("reveal/choose/discard role chain should parse");
    let coercion_debug = format!("{:#?}", coercion_def.spell_effect);
    let compact_coercion_debug = coercion_debug.split_whitespace().collect::<String>();
    assert!(
        compact_coercion_debug.contains("LookAtHandEffect")
            && compact_coercion_debug.contains("Target(Player(Opponent")
            && compact_coercion_debug.contains("ChooseObjectsEffect")
            && compact_coercion_debug.contains("chooser:You")
            && compact_coercion_debug.contains("DiscardEffect")
            && compact_coercion_debug.contains("player:Target(Opponent"),
        "expected target opponent as affected player and you as chooser, got {coercion_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn player_subject_role_boundary_regressions_for_search_choose_and_cast() {
    for (name, text, expected) in [
        (
            "You Search Opponent Library",
            "You search target opponent's library for a card and exile it. Then that player shuffles.",
            &[
                "ChooseObjectsEffect",
                "owner:Some(Target(Opponent",
                "chooser:You",
                "ShuffleLibraryEffect",
                "player:Target(Target(Opponent",
            ][..],
        ),
        (
            "Choose Opponent Graveyard Card",
            "Choose a card from target opponent's graveyard. Exile that card.",
            &[
                "ChooseObjectsEffect",
                "owner:Some(Target(Opponent",
                "chooser:You",
                "MoveToZoneEffect",
                "zone:Exile",
            ][..],
        ),
        (
            "Opponent Chooses Your Creature",
            "Target opponent chooses a creature you control.",
            &[
                "ChooseObjectsEffect",
                "controller:Some(You",
                "chooser:Target(Opponent",
            ][..],
        ),
        (
            "That Player Casts Exiled Card",
            "Exile the top card of target opponent's library. That player may cast it this turn.",
            &[
                "ExileTopOfLibraryEffect",
                "player:Target(Opponent",
                "GrantPlayTaggedEffect",
            ][..],
        ),
        (
            "You Cast Exiled Card",
            "Exile the top card of target opponent's library. You may cast it this turn.",
            &[
                "ExileTopOfLibraryEffect",
                "player:Target(Opponent",
                "GrantPlayTaggedEffect",
                "player:You",
            ][..],
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .parse_text(text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"));
        let debug = format!("{:#?}", def.spell_effect);
        let compact = debug.split_whitespace().collect::<String>();
        for expected_fragment in expected {
            assert!(
                compact.contains(expected_fragment),
                "expected {name} to contain {expected_fragment}, got {debug}"
            );
        }
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_damage_to_each_creature_equal_to_devotion() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Skyreaping")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Skyreaping deals damage to each creature with flying equal to your devotion to green.",
        )
        .expect("devotion damage to each matching creature should parse");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Deal damage to each creature with flying equal to your devotion to green."
    );
    let debug = format!("{:?}", def.spell_effect.expect("spell effect"));
    assert!(
        debug.contains("ForEachObject")
            && debug.contains("Devotion")
            && debug.contains("Flying")
            && debug.contains("target: Iterated"),
        "expected devotion damage to be applied to each flying creature, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_target_source_power_damage_to_other_target_and_itself() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Self-Destruct")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Target creature you control deals X damage to any other target and X damage to itself, where X is its power.",
        )
        .expect("source power damage split between another target and itself should parse");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Target creature you control deals X damage to any other target and X damage to itself, where X is its power."
    );
    let debug = format!("{:?}", def.spell_effect.expect("spell effect"));
    assert!(
        debug.contains("TargetOnlyEffect")
            && debug.contains("targeted_0")
            && debug.contains("ExecuteWithSourceEffect")
            && debug.contains("AnyOtherTarget")
            && debug.contains("PowerOf"),
        "expected one target source reused for both power damage events, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tapped_this_way_count_damage_from_triggered_tap_effect() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Monsoon")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each player's end step, tap all untapped Islands that player controls and this enchantment deals X damage to the player, where X is the number of Islands tapped this way.",
        )
        .expect("triggered tap/damage with tapped-this-way count should parse");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "At the beginning of each player's end step, tap all untapped Islands that player controls and this enchantment deals X damage to the player, where X is the number of Islands tapped this way."
    );
    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("WithIdEffect")
            && debug.contains("TapEffect")
            && debug.contains("EffectValue")
            && debug.contains("Active"),
        "expected damage to use the tap effect result and hit the active player, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_chosen_creature_type_x_boost_as_choice_inline() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tribal Unity")
        .card_types(vec![CardType::Instant])
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::X],
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Green],
        ]))
        .parse_text("Creatures of the creature type of your choice get +X/+X until end of turn.")
        .expect("chosen creature type X boost should parse");
    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Creatures of the creature type of your choice get +X/+X until end of turn."
    );
    let debug = format!("{:?}", def.spell_effect.expect("spell effect"));
    assert!(
        debug.contains("ChooseCreatureTypeEffect")
            && debug.contains("chosen_creature_type: true")
            && debug.contains("ModifyPowerToughness"),
        "expected a creature-type choice feeding a chosen-type X boost, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_asinine_antics_uses_flash_cast_method_and_iterated_role_attachment() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Asinine Antics")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You may cast this spell as though it had flash if you pay {2} more to cast it.\n\
             For each creature your opponents control, create a Cursed Role token attached to that creature.",
        )
        .expect("Asinine Antics should parse");

    assert_eq!(
        def.alternative_casts.len(),
        1,
        "flash timing with an extra cost should be a cast method, not a spell effect"
    );
    match &def.alternative_casts[0] {
        AlternativeCastingMethod::FlashWithAdditionalCost {
            additional_cost,
            total_cost,
        } => {
            assert_eq!(additional_cost.to_oracle(), "{2}");
            assert_eq!(
                total_cost.mana_cost().map(ManaCost::to_oracle).as_deref(),
                Some("{2}{U}{U}{2}"),
                "flash cast should use the printed cost plus the extra payment"
            );
        }
        other => panic!("expected flash-with-additional-cost method, got {other:?}"),
    }

    let spell_effect = def.spell_effect.as_ref().expect("spell effect");
    let debug = format!("{spell_effect:?}");
    assert!(
        !debug.contains("MayEffect") && !debug.contains("PayManaEffect"),
        "the flash extra payment should not become a resolution-time may-pay effect: {debug}"
    );
    assert!(
        debug.contains("ForEachObject")
            && debug.contains("CreateTokenEffect")
            && debug.contains("Cursed Role")
            && debug.contains("SetBasePowerToughness")
            && debug.contains("AttachObjectsEffect")
            && debug.contains("target: Iterated"),
        "role creation should iterate opponent creatures and attach each token to that iterated creature, got {debug}"
    );

    let rendered = canonical_compiled_lines(&def).join("\n");
    assert!(
        rendered.starts_with(
            "You may cast this spell as though it had flash if you pay {2} more to cast it"
        ),
        "expected flash extra-cost permission to render before the spell effect, got {rendered}"
    );
    assert!(
        rendered.contains(
            "You may cast this spell as though it had flash if you pay {2} more to cast it"
        ),
        "expected oracle-like flash extra-cost line, got {rendered}"
    );
    assert!(
        rendered.contains(
            "For each creature your opponents control, create a Cursed Role token attached to that creature"
        ),
        "expected compact role attachment line, got {rendered}"
    );

    let unprocessed = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        unprocessed.contains(
            "For each creature your opponents control, create a Cursed Role token attached to that creature"
        ),
        "expected unprocessed scoring text to compact the role attachment, got {unprocessed}"
    );
    assert!(
        !unprocessed.contains("Attach it to that object"),
        "role attachment should render as part of token creation, got {unprocessed}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_flash_extra_cost_without_fixture_mana_cost_still_renders_source_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flash Extra Cost Fixture")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "You may cast this spell as though it had flash if you pay {2} more to cast it.",
        )
        .expect("parser fixtures without mana costs should still parse flash extra-cost text");

    match &def.alternative_casts[0] {
        AlternativeCastingMethod::FlashWithAdditionalCost {
            additional_cost,
            total_cost,
        } => {
            assert_eq!(additional_cost.to_oracle(), "{2}");
            assert_eq!(
                total_cost.mana_cost().map(ManaCost::to_oracle).as_deref(),
                Some("{2}"),
                "a fixture with no printed cost should preserve the extra payment as the available cast cost"
            );
        }
        other => panic!("expected flash-with-additional-cost method, got {other:?}"),
    }

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "You may cast this spell as though it had flash if you pay {2} more to cast it"
        ),
        "expected source-surface flash extra-cost rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_gold_token_uses_compact_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gild Variant")
        .parse_text("Exile target creature. Create a Gold token.")
        .expect("gold token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    assert!(
        joined.contains("Create a Gold token") || joined.contains("create a Gold token"),
        "expected compact gold token wording, got {joined}"
    );
    assert!(
        !joined
            .to_ascii_lowercase()
            .contains("unsupported parser line fallback"),
        "gold token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_shard_token_includes_scry_draw_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Niko Variant")
        .parse_text("When this permanent enters, create two Shard tokens.")
        .expect("shard token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("shard"),
        "expected shard token wording in compiled text, got {joined}"
    );
    assert!(
        lower.contains("scry 1") && lower.contains("draw a card"),
        "expected shard token rules text to include scry and draw, got {joined}"
    );
    assert!(
        !lower.contains("unsupported parser line fallback"),
        "shard token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_walker_token_uses_expected_characteristics() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Walker Maker Variant")
        .parse_text("Create three Walker tokens.")
        .expect("walker token creation should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n");
    let lower = joined.to_ascii_lowercase();
    assert!(
        lower.contains("walker") && lower.contains("2/2") && lower.contains("zombie"),
        "expected walker token characteristics in compiled text, got {joined}"
    );
    assert!(
        !lower.contains("unsupported parser line fallback"),
        "walker token parsing should not rely on unsupported fallback marker, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unprocessed_compiled_lines_compact_each_opponent_discard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Burglar Rat Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, each opponent discards a card.")
        .expect("etb discard should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("when this creature enters"),
        "expected source subject to stay creature-like, got {joined}"
    );
    assert!(
        joined.contains("each opponent discards a card"),
        "expected compact each-opponent discard wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unprocessed_compiled_lines_compact_you_mill_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Armored Skaab Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, mill four cards.")
        .expect("etb mill should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("when this creature enters"),
        "expected source subject to stay creature-like, got {joined}"
    );
    assert!(
        joined.contains("mill 4 cards")
            || joined.contains("mill four cards")
            || joined.contains("you mill four cards"),
        "expected compact mill wording without explicit 'you', got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unprocessed_compiled_lines_compact_cant_block_this_turn() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lambholt Harrier Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("{3}{R}: Target creature can't block this turn.")
        .expect("can't-block activated ability should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("target creature can't block this turn"),
        "expected oracle-like can't-block wording, got {joined}"
    );
    assert!(
        !joined.contains("choose target creature"),
        "target-only preface should be compacted away, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unprocessed_compiled_lines_compact_prevent_damage_source_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ordruun Commando Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{W}: Prevent the next 1 damage that would be dealt to this creature this turn.",
        )
        .expect("prevent damage activated ability should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("prevent the next 1 damage that would be dealt to this creature this turn"),
        "expected oracle-like prevention wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unprocessed_compiled_lines_compact_lands_have_tap_for_any_color() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Joiner Adept Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Lands you control have \"{T}: Add one mana of any color.\"")
        .expect("mana-grant static ability should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("lands you control have \"{t}: add one mana of any color\"")
            || joined.contains("lands you control have \"{t}: add one mana of any color.\""),
        "expected quoted tap-mana grant wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn unprocessed_compiled_lines_preserve_negative_zero_toughness_delta() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cumber Stone Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Creatures your opponents control get -1/-0.")
        .expect("static debuff should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("get -1/-0"),
        "expected oracle-like -1/-0 rendering, got {joined}"
    );
    assert!(
        joined.contains("creatures your opponents control get -1/-0"),
        "expected oracle-like opponent-controller wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_target_creature_or_vehicle_uses_union_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Daring Demolition Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature or Vehicle.")
        .expect("creature-or-vehicle targeting should parse");

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("DestroyEffect"),
        "expected destroy effect, got {debug}"
    );
    assert!(
        debug.contains("type_or_subtype_union: true"),
        "expected type/subtype union for creature-or-vehicle targeting, got {debug}"
    );
    assert!(
        debug.contains("card_types: [") && debug.contains("Creature"),
        "expected creature card type selector, got {debug}"
    );
    assert!(
        debug.contains("subtypes: [") && debug.contains("Vehicle"),
        "expected Vehicle subtype selector, got {debug}"
    );

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("Destroy target creature or Vehicle"),
        "expected oracle-like creature-or-Vehicle rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_multi_sacrifice_cost_uses_compact_filter_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Keldon Arsonist Variant")
        .parse_text("{1}, Sacrifice two lands: Destroy target land.")
        .expect("multi-sacrifice activated cost should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice two lands"),
        "expected compact multi-sacrifice rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_multi_sacrifice_artifacts_cost_uses_compact_filter_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Krark-Clan Engineers Variant")
        .parse_text("{R}, Sacrifice two artifacts: Destroy target artifact.")
        .expect("multi-artifact-sacrifice activated cost should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice two artifacts"),
        "expected compact multi-artifact sacrifice rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_single_sacrifice_cost_does_not_duplicate_article() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Greta Variant")
        .parse_text("{G}, Sacrifice a Food you control: Draw a card.")
        .expect("single sacrifice activated cost should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice a food you control"),
        "expected oracle-like singular sacrifice rendering, got {joined}"
    );
    assert!(
        !joined.contains("sacrifice a a food"),
        "expected no duplicated article in sacrifice rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_subtype_sacrifice_cost_uses_oracle_like_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin Variant")
        .parse_text("{R}, Sacrifice a Goblin: This creature deals 2 damage to any target.")
        .expect("single goblin sacrifice activated cost should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        joined.contains("{r}, sacrifice a goblin you control:"),
        "expected oracle-like goblin sacrifice rendering, got {joined}"
    );
    assert!(
        !joined.contains("choose exactly 1"),
        "expected sacrifice choice scaffolding to be hidden, got {joined}"
    );
    assert!(
        !joined.contains("sacrifice a permanent"),
        "expected generic sacrifice fallback to be hidden, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_multi_subtype_sacrifice_cost_uses_oracle_like_surface() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Goblin Warrens Variant")
        .parse_text("{2}{R}, Sacrifice two Goblins: Create three 1/1 red Goblin creature tokens.")
        .expect("multi-goblin sacrifice activated cost should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        joined.contains("{2}{r}, sacrifice two goblins you control:"),
        "expected oracle-like multi-goblin sacrifice rendering, got {joined}"
    );
    assert!(
        !joined.contains("sacrifice two permanent"),
        "expected generic multi-sacrifice fallback to be hidden, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_sacrifice_artifact_or_land_cost_uses_oracle_article() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scrapchomper Variant")
        .parse_text("{1}{R}, {T}, Sacrifice an artifact or land: Draw a card.")
        .expect("artifact-or-land sacrifice activated cost should parse");
    let lines = crate::compiled_text::unprocessed_compiled_lines(&def);
    let joined = lines.join("\n").to_ascii_lowercase();
    assert!(
        joined.contains("sacrifice an artifact or land"),
        "expected oracle-like sacrifice article rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_scheming_symmetry_keeps_targeted_players_and_search_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scheming Symmetry Variant")
        .parse_text(
            "Choose two target players. Each of them searches their library for a card, then shuffles and puts that card on top.",
        )
        .expect("scheming symmetry should parse");
    let debug = format!("{:?}", def.spell_effect);
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        (debug.contains("SearchLibraryEffect") || debug.contains("ChooseObjectsEffect"))
            && debug.contains("chooser: IteratedPlayer")
            && (debug.contains("player: IteratedPlayer")
                || debug.contains("owner: Some(IteratedPlayer)"))
            && (debug.contains("destination: Library") || debug.contains("zone: Library")),
        "expected compact per-target library search effect, got {debug}"
    );
    assert!(
        joined.contains("choose two target players"),
        "expected chosen target players to remain visible, got {joined}; debug={debug}"
    );
    assert!(
        joined.contains(
            "choose two target players. each of them searches their library for a card, then shuffles and puts that card on top"
        ) && !joined.contains("for each target player, that player"),
        "expected unambiguous per-target-player search rendering, got {joined}"
    );
    assert!(
        !joined.contains("you searches"),
        "expected target-player carry instead of defaulting to you, got {joined}"
    );
    assert!(
        !joined.contains("in any order"),
        "expected single-card top placement to avoid in-any-order fallback, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_varragoth_search_uses_the_searched_card_antecedent() {
    let def = parse_oracle_card_definition("Varragoth, Bloodsky Sire");
    let joined = compiled_text_lines(&def).join("\n").to_ascii_lowercase();

    assert!(
        joined.contains(
            "target player searches their library for a card, then shuffles and puts that card on top"
        ) && !joined.contains("puts the card on top"),
        "expected the searched-card antecedent to remain explicit, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_scholarship_sponsor_keeps_each_player_search_subject() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Scholarship Sponsor Variant")
        .parse_text(
            "When this creature enters, each player who controls fewer lands than the player who controls the most lands searches their library for a number of basic land cards less than or equal to the difference, puts those cards onto the battlefield tapped, then shuffles.",
        )
        .expect("scholarship sponsor should parse");
    let debug = format!("{:?}", def.spell_effect);
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        !joined.contains("you searches"),
        "expected per-player search subject instead of defaulting to you, got {joined}; debug={debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_rock_slide_distributed_damage_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Rock Slide Variant")
        .parse_text(
            "Rock Slide deals X damage divided as you choose among any number of target attacking or blocking creatures without flying.",
        )
        .expect("rock slide should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "deal x damage divided as you choose among any number of target attacking or blocking creatures without flying"
        ),
        "expected distributed-damage rendering, got {joined}"
    );
    assert!(
        !joined.contains("unsupported effect"),
        "expected supported distributed-damage rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn fabricate_cards_stay_semantically_aligned_with_compiled_lines() {
    for (name, oracle) in [
        (
            "Visionary Augmenter Variant",
            "Fabricate 2 (When this creature enters, put two +1/+1 counters on it or create two 1/1 colorless Servo artifact creature tokens.)",
        ),
        (
            "Weaponcraft Enthusiast Variant",
            "Fabricate 2 (When this creature enters, put two +1/+1 counters on it or create two 1/1 colorless Servo artifact creature tokens.)",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::new(), name)
            .parse_text(oracle)
            .expect("fabricate card should parse");
        let compiled = crate::compiled_text::unprocessed_compiled_lines(&def);
        let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
            crate::semantic_compare::compare_semantics_scored(
                oracle,
                &compiled,
                Some(crate::semantic_compare::EmbeddingConfig {
                    dims: 384,
                    mismatch_threshold: 0.99,
                }),
            );
        assert!(
            similarity >= 0.99,
            "expected fabricate compiled lines to preserve semantics, got score={similarity}, lines={compiled:?}"
        );
        assert!(
            !mismatch,
            "expected fabricate compiled lines to avoid mismatch, got lines={compiled:?}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_return_from_graveyard_uses_from_your_graveyard() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Reanimate Variant")
        .parse_text("Return target creature card from your graveyard to the battlefield.")
        .expect("return-from-graveyard spell should parse");
    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Return target creature card from your graveyard to the battlefield"),
        "expected oracle-like return text, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_return_from_graveyard_attached_followup_targets_returned_creature() {
    let text = "Return target creature card from your graveyard to the battlefield, then return up to two target Aura and/or Equipment cards from your graveyard to the battlefield attached to that creature.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Unfinished Business Variant")
        .parse_text(text)
        .expect("return-attached followup should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ReturnFromGraveyardToBattlefieldEffect")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("AttachObjectsEffect")
            && debug.contains("Aura")
            && debug.contains("Equipment")
            && debug.contains("max: Some(2)")
            && debug.contains("TagKey(\"returned_"),
        "expected returned creature tag plus counted Aura/Equipment move+attach, got {debug}"
    );

    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        joined.contains(text.trim_end_matches('.')),
        "expected compact return-attached wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_return_to_hand_from_your_graveyard_uses_oracle_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Raise Dead Variant")
        .parse_text("Return target creature card from your graveyard to your hand.")
        .expect("return-to-hand-from-graveyard spell should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("Return target creature card from your graveyard to your hand"),
        "expected oracle-like return-to-hand wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_graveyard_self_return_activated_uses_this_card_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sanitarium Skeleton Variant")
        .parse_text("{2}{B}: Return this card from your graveyard to your hand.")
        .expect("graveyard self-return activated ability should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("{2}{B}: Return this card from your graveyard to your hand"),
        "expected oracle-like graveyard self-return wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_enchanted_tap_untap_compacts_tag_prelude() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Freed from the Real Variant")
        .parse_text(
            "Enchant creature\n{U}: Tap enchanted creature.\n{U}: Untap enchanted creature.",
        )
        .expect("enchanted tap/untap aura should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    let lower = joined.to_ascii_lowercase();
    assert!(
        (lower.contains("tap enchanted creature")
            || lower.contains("tap enchanted permanent")
            || lower.contains("tap an enchanted creature"))
            && (lower.contains("untap enchanted creature")
                || lower.contains("untap enchanted permanent")
                || lower.contains("untap an enchanted creature")),
        "expected compact enchanted tap/untap wording, got {joined}"
    );
    assert!(
        !lower.contains("tag the object attached to this source")
            && !lower.contains("the tagged object 'enchanted'"),
        "internal enchanted tag prelude should not leak into oracle-like lines: {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_draw_then_put_two_cards_from_hand_on_top_preserves_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Brainstorm Variant")
        .parse_text("Draw three cards, then put two cards from your hand on top of your library in any order.")
        .expect("draw-then-put-two-cards clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw three cards")
            && rendered.contains("put two cards from your hand on top of your library"),
        "expected draw-then-put-two-cards wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_each_player_puts_card_from_hand_on_top_normalizes_for_each_form() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sadistic Augermage Variant")
        .parse_text("When this creature dies, each player puts a card from their hand on top of their library.")
        .expect("each-player hand-to-library clause should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("each player puts a card from their hand on top of their library"),
        "expected normalized each-player hand-to-library wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_all_slivers_have_regenerate_uses_quoted_ability_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Poultice Sliver Variant")
        .parse_text("All Slivers have \"{2}, {T}: Regenerate target Sliver.\"")
        .expect("all-slivers-regenerate line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("All Slivers have \"{2}, {T}: Regenerate target Sliver.\"")
            || joined.contains("All Slivers have \"{2}, {T}: Regenerate target sliver.\"")
            || joined.contains("All Sliver creatures have \"{2}, {T}: Regenerate target sliver.\"")
            || joined.contains("All Sliver creatures have \"{2}, {T}: Regenerate target Sliver.\""),
        "expected quoted Sliver granted ability wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_all_slivers_have_sacrifice_add_mana_uses_quoted_ability_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Basal Sliver Variant")
        .parse_text("All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"")
        .expect("all-slivers-sacrifice-mana line should parse");
    let joined = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert!(
        joined.contains("All Slivers have \"Sacrifice this permanent: Add {B}{B}.\"")
            || joined
                .contains("All Sliver creatures have \"Sacrifice this permanent: Add {B}{B}.\""),
        "expected quoted Sliver sacrifice-mana wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_surveil_uses_keyword_action_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Surveil Variant")
        .parse_text("Surveil 1.")
        .expect("surveil spell should parse");
    let lines = unprocessed_compiled_lines(&def);
    let spell_line = lines.join(" ");
    assert!(
        spell_line.contains("Surveil 1"),
        "expected oracle-like surveil text, got {spell_line}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tap_target_spirit_uses_subtype_noun() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Spirit Tapper Variant")
        .parse_text("{T}: Tap target Spirit.")
        .expect("tap target Spirit should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target spirit"),
        "expected Spirit subtype noun rendering, got {joined}"
    );
    assert!(
        !joined.contains("permanent spirit"),
        "unexpected permanent noun for Spirit subtype, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tap_target_wall_uses_subtype_noun() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wall Tapper Variant")
        .parse_text("{R}: Tap target Wall.")
        .expect("tap target Wall should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target wall"),
        "expected Wall subtype noun rendering, got {joined}"
    );
    assert!(
        !joined.contains("permanent wall"),
        "unexpected permanent noun for Wall subtype, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_untap_target_snow_land_includes_supertype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Snow Untapper Variant")
        .parse_text("{T}: Untap target snow land.")
        .expect("untap target snow land should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("snow land"),
        "expected snow supertype rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_artifacts_and_lands_enter_tapped_uses_union_types() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Root Maze Variant")
        .parse_text("Artifacts and lands enter the battlefield tapped.")
        .expect("artifacts and lands enter tapped should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("artifacts and lands enter tapped"),
        "expected union type rendering, got {joined}"
    );
    assert!(
        !joined.contains("artifact land enter the battlefield tapped"),
        "unexpected artifact-land intersection rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_damage_each_creature_and_each_player_keeps_both_targets() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Steam Blast Variant")
        .parse_text("This spell deals 2 damage to each creature and each player.")
        .expect("damage each creature and each player should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("each player"),
        "expected player damage target in rendering, got {joined}"
    );
    assert!(
        joined.contains("each creature"),
        "expected creature damage target in rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_subject_with_counters_cant_be_blocked_preserves_subject_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Herald Variant")
        .parse_text("Creatures you control with +1/+1 counters on them can't be blocked.")
        .expect("subject unblockable static line should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("creatures you control") && joined.contains("can't be blocked"),
        "expected subject + restriction rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_granted_counter_subject_preserves_counter_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hagra Variant")
        .parse_text(
            "This creature enters with two +1/+1 counters on it.\nEach creature you control with a +1/+1 counter on it has menace.",
        )
        .expect("counter-qualified grant line should parse");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("menace"),
        "expected counter-qualified menace grant rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("permanents have menace"),
        "rendering regressed to broad permanent grant: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_subject_with_power_or_toughness_cant_be_blocked_preserves_filter() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Tetsuko Variant")
        .parse_text("Creatures you control with power or toughness 1 or less can't be blocked.")
        .expect_err("power/toughness unblockable static line should fail loudly");
    let joined = format!("{err:?}").to_ascii_lowercase();
    assert!(
        joined.contains("unsupported power-or-toughness cant-be-blocked subject"),
        "expected explicit unsupported parse error, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_create_saproling_token_keeps_subtype() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sprout Variant")
        .parse_text("Create a 1/1 green Saproling creature token.")
        .expect("saproling token text should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("saproling"),
        "expected Saproling subtype in rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_mount_or_vehicle_target() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Daring Mechanic Variant")
        .parse_text("{3}{W}: Put a +1/+1 counter on target Mount or Vehicle.")
        .expect("mount or vehicle target should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("target mount or vehicle"),
        "expected mount or vehicle target rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tap_cost_ability_filter_phrase() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Magewright Stone Variant")
        .parse_text(
            "{1}, {T}: Untap target creature that has an activated ability with {T} in its cost.",
        )
        .expect("tap-cost activated-ability filter should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("untap target creature that has an activated ability with {t} in its cost"),
        "expected activated-ability tap-cost filter rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_enchanted_creatures_you_control_pluralizes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "A Tale Variant")
        .parse_text("Enchanted creatures you control get +2/+2.")
        .expect("enchanted-creature anthem should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("enchanted creatures you control get +2/+2"),
        "expected plural enchanted creatures rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_bonehoard_static_bonus_mentions_all_graveyards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bonehoard Variant")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "Living weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)\n\
             Equipped creature gets +X/+X, where X is the number of creature cards in all graveyards.\n\
             Equip {2}",
        )
        .expect("Bonehoard text should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("living weapon")
            && joined.contains(
                "equipped creature gets +x/+x, where x is the number of creature cards in all graveyards"
            )
            && joined.contains("equip {2}"),
        "expected Bonehoard to render the all-graveyards bonus correctly, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_kembas_banner_equipped_bonus_uses_for_each_wording() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kemba's Banner Variant")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "For Mirrodin! (When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to it.)\n\
             Equipped creature gets +1/+1 for each creature you control.\n\
             Equip {2}{W}",
        )
        .expect("Kemba's Banner text should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("for mirrodin!")
            && joined.contains("equipped creature gets +1/+1 for each creature you control")
            && joined.contains("equip {2}{w}"),
        "expected Kemba's Banner to preserve the oracle-style for-each wording, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hexplate_wallbreaker_parses_and_renders_first_combat_phase_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(605_584), "Hexplate Wallbreaker")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(3)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::Red],
        ]))
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Equipment])
        .parse_text(
            "For Mirrodin! (When this Equipment enters, create a 2/2 red Rebel creature token, then attach this to it.)\n\
             Equipped creature gets +2/+2.\n\
             Whenever equipped creature attacks, if it's the first combat phase of the turn, untap each attacking creature. After this phase, there is an additional combat phase.\n\
             Equip {3}{R}",
        )
        .expect("Hexplate Wallbreaker should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        !lower.contains("unsupported predicate") && !lower.contains("unsupported effect"),
        "Hexplate Wallbreaker should not fall back to unsupported output, got {rendered}"
    );
    assert!(
        lower.contains("for mirrodin!")
            && lower.contains("equipped creature gets +2/+2")
            && lower.contains("whenever equipped creature attacks")
            && lower.contains("if it's the first combat phase of the turn")
            && lower.contains("untap each attacking creature")
            && lower.contains("there is an additional combat phase")
            && lower.contains("equip {3}{r}"),
        "expected Hexplate Wallbreaker compiled text to preserve its equipment trigger, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_static_bonus_preserves_creature_type_among_scope() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kindred Scout Variant")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Shapeshifter])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text("This creature gets +1/+1 for each creature type among creatures you control.")
        .expect("creature-type static bonus should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains(
            "this creature gets +1/+1 for each creature type among creatures you control"
        ),
        "expected creature-type among scope to be preserved, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_allies_you_control_pluralizes() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Allied Teamwork Variant")
        .parse_text("Allies you control get +1/+1.")
        .expect("allies anthem should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("allies you control get +1/+1"),
        "expected plural allies rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tap_or_untap_mode_compacts() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hyperion Blacksmith Variant")
        .parse_text("{T}: You may tap or untap target artifact an opponent controls.")
        .expect("tap or untap should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("tap or untap target opponent's artifact")
            || joined.contains("tap or untap target artifact an opponent controls"),
        "expected compact tap/untap rendering, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_tap_or_untap_mode_does_not_compact_when_targets_differ() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deceiver Exarch Variant")
        .parse_text(
            "When this creature enters, choose one —\n• Untap target permanent you control.\n• Tap target permanent an opponent controls.",
        )
        .expect("modal tap/untap with different targets should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !joined.contains("tap or untap target"),
        "different tap/untap targets should not compact, got {joined}"
    );
    assert!(
        joined.contains("choose one")
            && joined.contains("untap target permanent you control")
            && joined.contains("tap target permanent an opponent controls"),
        "expected separate modal tap/untap lines, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn oracle_like_equipped_sacrifice_uses_card_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ninja's Kunai")
        .parse_text(
            "Type: Artifact — Equipment\nEquipped creature has \"{1}, {T}, Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target.\"\nEquip {1}",
        )
        .expect("ninja's kunai should parse");
    let lines = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        lines.contains("Sacrifice Ninja's Kunai: Ninja's Kunai deals 3 damage to any target")
            || lines.contains("Sacrifice this: This deals 3 damage to any target"),
        "expected equipment self-reference rendering, got {lines}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_search_library_for_card_uses_card_noun() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Search Variant")
        .parse_text(
            "Search your library for a card, reveal it, put it into your hand, then shuffle.",
        )
        .expect("search clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("search your library for a card"),
        "expected search filter to render as card, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_standalone_shuffle_clause_defaults_to_library_owner() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Shuffle Variant")
        .parse_text("Search your library for a card, put it into your hand. Shuffle.")
        .expect("standalone shuffle clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("search your library for a card")
            && joined.contains("shuffle your library"),
        "expected standalone shuffle to resolve to your library, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_target_player_library_and_exile_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Denying Wind Variant")
        .parse_text("Search target player's library for up to seven cards and exile them. Then that player shuffles.")
        .expect("target-player search-and-exile clause should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("search target player's library for up to 7 cards and exile them. then that player shuffles"),
        "expected search/exile/shuffle rendering, got {joined}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(Library)")
            && debug.contains("zone: Exile"),
        "expected search-from-library into exile sequence, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn nightmare_incursion_uses_you_as_search_chooser_and_binds_where_x_count() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Nightmare Incursion Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search target player's library for up to X cards, where X is the number of Swamps you control, and exile them. Then that player shuffles.",
        )
        .expect("Nightmare Incursion variant should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("search target player's library for up to x cards")
            && joined.contains("where x is the number of swamps you control")
            && joined.contains("and exile them")
            && joined.contains("then that player shuffles")
            && !joined.contains("exile target player"),
        "expected Nightmare Incursion search/exile/shuffle wording, got {joined}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("chooseobjectseffect")
            && debug.contains("count_value: some(")
            && debug.contains("count(")
            && debug.contains("subtypes: [swamp]")
            && debug.contains("zone: exile")
            && debug.contains("shufflelibraryeffect")
            && debug.contains("target(player(target(any)))"),
        "expected Nightmare Incursion to search target player's library, exile found cards, and bind count to swamps you control, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_its_controller_graveyard_hand_and_library_exiles_same_name_cards() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quash Variant")
        .parse_text("Counter target spell. Search its controller's graveyard, hand, and library for all cards with the same name as that spell and exile them. Then that player shuffles.")
        .expect("multi-zone same-name search-and-exile clause should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !joined.contains("exile target player"),
        "search clause must not collapse into exile-player fallback, got {joined}"
    );
    assert!(
        joined.contains("search its controller's graveyard, hand, and library for all cards with the same name as that object and exile them")
            && joined.contains("that player shuffles"),
        "expected compact multi-zone search rendering, got {joined}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("additional_zones: [hand, library]")
            && (debug.contains("zone: graveyard") || debug.contains("zone: none"))
            && debug.contains("samenameastagged")
            && debug.contains("controllerof")
            && debug.contains("shufflelibraryeffect"),
        "expected same-name exile across hand/graveyard/library and controller shuffle, got {debug}"
    );
    assert_eq!(
        debug.matches("shufflelibraryeffect").count(),
        1,
        "expected exactly one shuffle, got {debug}"
    );
    assert!(
        debug.contains("shufflelibraryeffect { player: controllerof(target)")
            || debug
                .contains("shufflelibraryeffect { player: controllerof(tagged(tagkey(\"exiled_"),
        "expected shuffle to target the searched player's controller, got {debug}"
    );
}

#[test]
pub(super) fn parse_invasive_surgery_oracle_strict_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Invasive Surgery");

    let rendered = compiled_text_lines(&def).join(" ");
    let debug = format!("{:?}", def.spell_effect);
    assert!(
        rendered.contains("Counter target sorcery spell")
            && rendered.contains(
                "Delirium — If there are four or more card types among cards in your graveyard"
            )
            && rendered
                .contains("search the graveyard, hand, and library of that spell's controller")
            && rendered.contains("for any number of cards with the same name as that spell")
            && rendered.contains("exile those cards, then that player shuffles"),
        "expected Invasive Surgery compiled text to preserve counter, delirium, optional same-name multi-zone search, and shuffle, got {rendered}\n{debug}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("unsupported")
            && !rendered.contains("sorcery spell spell")
            && !rendered.contains("for all cards with the same name"),
        "Invasive Surgery should parse strictly without fallback or over-broad all-cards wording, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("conditionaleffect")
            && debug.contains("playerhascardtypesingraveyardormore")
            && debug.contains("samenameastagged")
            && debug.contains("controllerof")
            && debug.contains("additional_zones: [hand, library]")
            && debug.matches("shufflelibraryeffect").count() == 1,
        "expected Invasive Surgery to lower to one delirium-gated controller same-name search/exile/shuffle, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_reap_intellect_regression() {
    let def = parse_oracle_card_definition("Reap Intellect");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        rendered.contains("target opponent reveals their hand"),
        "expected Reap Intellect to reveal target opponent's hand, got {rendered}"
    );
    assert!(
        rendered.contains("choose up to x nonland cards from it and exile them")
            || rendered.contains("choose up to x nonland card from it and exile them"),
        "expected Reap Intellect to keep the hand-choice exile clause, got {rendered}; debug={debug}"
    );
    assert!(
        (rendered.contains("search that player's graveyard, hand, and library")
            || rendered.contains("search target opponent's graveyard, hand, and library"))
            && (rendered.contains("with the same name as that object")
                || rendered.contains("with the same name as that card")
                || rendered.contains("with the same name as those cards"))
            && rendered.contains("exile them")
            && (rendered.contains("that player shuffles")
                || rendered.contains("shuffle target opponent's library")),
        "expected Reap Intellect to keep the same-name search/exile/shuffle follow-up, got {rendered}; debug={debug}"
    );
    assert!(
        !rendered.contains("exile target opponent")
            && !rendered.contains("you search your graveyard, hand, and library"),
        "expected Reap Intellect to avoid player-exile and wrong-owner fallback text, got {rendered}; debug={debug}"
    );

    assert!(
        debug.contains("lookathandeffect")
            && debug.contains("chooseobjectseffect")
            && debug.contains("zone: some(hand)")
            && debug.contains("samenameastagged")
            && debug.contains("shufflelibraryeffect"),
        "expected Reap Intellect to lower to hand choice plus same-name search, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_card_name_then_draw_for_each_card_exiled_from_hand_this_way() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Stone Brain Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Choose a card name. Search target opponent's graveyard, hand, and library for all cards with that name and exile them. Then that player shuffles, then draws a card for each card exiled from their hand this way.",
        )
        .expect("stone brain style clause should parse");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("choosecardnameeffect")
            && debug.contains("drawforeachtaggedmatchingeffect")
            && debug.contains("shufflelibraryeffect"),
        "expected choose-name search/exile/draw lowering, got {debug}"
    );
}

#[test]
pub(super) fn parse_predict_oracle_strict_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Predict");

    let rendered = canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Choose a card name, then target player mills a card")
            && rendered.contains("If a card with the chosen name was milled this way")
            && rendered.contains("Otherwise, draw a card"),
        "expected Predict compiled text to preserve choose-name mill condition, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("ChooseCardNameEffect")
            && debug.contains("MillEffect")
            && debug.contains("ConditionalEffect")
            && debug.contains("SameNameAsTagged"),
        "expected Predict to lower to choose-name, tagged mill, and same-name condition, got {debug}"
    );
}

#[test]
pub(super) fn predict_draws_two_when_milled_card_has_chosen_name() {
    let (alice_hand, bob_graveyard) = execute_predict_with_top_card("Brainstorm", "Brainstorm");

    assert_eq!(
        alice_hand, 2,
        "Predict should draw two when the milled card has the chosen name"
    );
    assert_eq!(
        bob_graveyard,
        vec!["Brainstorm".to_string()],
        "Predict should mill the target player's top card"
    );
}

#[test]
pub(super) fn predict_draws_one_when_milled_card_has_different_name() {
    let (alice_hand, bob_graveyard) = execute_predict_with_top_card("Brainstorm", "Opt");

    assert_eq!(
        alice_hand, 1,
        "Predict should draw one when the milled card does not have the chosen name"
    );
    assert_eq!(
        bob_graveyard,
        vec!["Opt".to_string()],
        "Predict should still mill the target player's top card"
    );
}

pub(super) fn execute_predict_with_top_card(
    chosen_name: &str,
    top_card_name: &str,
) -> (usize, Vec<String>) {
    struct PredictDecisionMaker {
        chosen_name: String,
    }

    impl crate::decision::DecisionMaker for PredictDecisionMaker {
        fn decide_text(
            &mut self,
            _game: &crate::GameState,
            _ctx: &crate::decisions::context::TextInputContext,
        ) -> String {
            self.chosen_name.clone()
        }
    }

    fn simple_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build()
    }

    let def = parse_oracle_card_definition("Predict");
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let mut game = crate::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let source = game.create_object_from_definition(&def, alice, Zone::Stack);

    game.create_object_from_card(&simple_card("Alice Draw One"), alice, Zone::Library);
    game.create_object_from_card(&simple_card("Alice Draw Two"), alice, Zone::Library);
    game.create_object_from_card(&simple_card(top_card_name), bob, Zone::Library);

    let mut dm = PredictDecisionMaker {
        chosen_name: chosen_name.to_string(),
    };
    let mut ctx = crate::effects::ExecutionContext::new(source, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        def.spell_effect.as_ref().expect("Predict spell effect"),
        None,
        &[],
    )
    .expect("Predict should resolve");

    let alice_hand = game.player(alice).expect("Alice").hand.len();
    let bob_graveyard = game
        .player(bob)
        .expect("Bob")
        .graveyard
        .iter()
        .filter_map(|id| game.object(*id).map(|object| object.name.to_string()))
        .collect::<Vec<_>>();
    (alice_hand, bob_graveyard)
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_destroy_then_search_target_opponent_library_preserves_destroy_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Life's Finale Variant")
        .parse_text("Destroy all creatures, then search target opponent's library for up to three creature cards and put them into their graveyard. Then that player shuffles.")
        .expect("destroy-then-search clause should parse");

    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("destroy all creatures")
            && joined.contains("search target opponent's library for up to three creature")
            && joined.contains("put them into")
            && joined.contains("graveyard")
            && (joined.contains("then that player shuffles")
                || joined.contains("shuffle target opponent's library")),
        "expected destroy and search/put/shuffle chain, got {joined}"
    );
    assert!(
        !joined.contains("destroy all creatures card in an opponent's libraries"),
        "search clause should not degrade into destroy-library fallback, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_where_x_is_fixed_plus_number_of_filter_value() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Muscle Burst Variant")
        .parse_text(
            "Target creature gets +X/+X until end of turn, where X is 3 plus the number of cards named Muscle Burst in all graveyards.",
        )
        .expect("where-X fixed-plus-count gets clause should parse");

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("modifypowertoughness")
            && debug.contains("add(fixed(3), count(")
            && debug.contains("name: some(\"muscle burst\")")
            && debug.contains("graveyard"),
        "expected fixed-plus-count where-X value in compiled effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_filter_artifact_with_mana_ability_or_basic_land() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Moonsilver Key Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text(
            "{1}, {T}, Sacrifice this artifact: Search your library for an artifact card with a mana ability or a basic land card, reveal it, put it into your hand, then shuffle.",
        )
        .expect("artifact-with-mana-ability-or-basic-land search should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("any_of: [ObjectFilter"),
        "expected disjunctive search filter branches, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("ability_markers: [\"mana ability\"]")
            && abilities_debug.contains("supertypes: [Basic]")
            && abilities_debug.contains("card_types: [Land]"),
        "expected mana-ability and basic-land branch constraints, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("artifact with mana ability or basic land"),
        "expected disjunctive search wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_filter_artifact_with_mana_cost_zero_or_one() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Urza's Saga Search Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for an artifact card with mana cost 0 or 1, put it onto the battlefield, then shuffle.",
        )
        .expect("artifact-with-mana-cost-zero-or-one search should parse");

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    let compact = spell_debug.split_whitespace().collect::<String>();
    assert!(
        compact.contains("sequenceeffect")
            && compact.contains("chooseobjectseffect")
            && compact.contains("any_of:[objectfilter")
            && compact.contains("card_types:[artifact")
            && compact.contains("has_mana_cost:true")
            && compact.contains("no_x_in_cost:true")
            && compact.contains("mana_value:some(equal(0")
            && compact.contains("mana_value:some(equal(1")
            && compact.contains("putontobattlefieldeffect")
            && compact.contains("shufflelibraryeffect"),
        "expected exact zero-or-one mana-cost search branches, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn render_powerstone_token_name() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Powerstone Variant")
        .parse_text("Create a tapped Powerstone token.")
        .expect("powerstone token clause should parse");
    let joined = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        joined.contains("powerstone token") && joined.contains("tapped"),
        "expected powerstone token name in compiled text, got {joined}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_the_mana_rig_oracle_text_strictly() {
    let oracle = "Whenever you cast a multicolored spell, create a tapped Powerstone token.\n{X}{X}{X}, {T}: Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order.";
    let def = CardDefinitionBuilder::new(CardId::new(), "The Mana Rig")
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("The Mana Rig should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Put up to two of them into your hand and the rest on the bottom of your library in a random order"),
        "expected looked-card split clause in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_the_mana_rig_tracks_trigger_and_xxx_tap_activation_shape() {
    let oracle = "Whenever you cast a multicolored spell, create a tapped Powerstone token.\n{X}{X}{X}, {T}: Look at the top X cards of your library. Put up to two of them into your hand and the rest on the bottom of your library in a random order.";
    let def = CardDefinitionBuilder::new(CardId::new(), "The Mana Rig")
        .card_types(vec![CardType::Artifact])
        .parse_text(oracle)
        .expect("The Mana Rig should parse strictly");

    let abilities_debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        abilities_debug.contains("multicolored") && abilities_debug.contains("powerstone"),
        "expected multicolored-cast trigger creating Powerstone token, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("manapaymentcost")
            && abilities_debug.contains("pips")
            && abilities_debug.contains("tapeffect")
            && abilities_debug.contains("choicecount")
            && abilities_debug.contains("min: 0")
            && abilities_debug.contains("max: some(")
            && abilities_debug.contains("2"),
        "expected XXX+tap activated ability with up-to-two looked-card choice, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_black_cat_cunning_thief_oracle_text_strictly() {
    let oracle = "When Black Cat enters, look at the top nine cards of target opponent's library, exile two of them face down, then put the rest on the bottom of their library in a random order. You may play the exiled cards for as long as they remain exiled. Mana of any type can be spent to cast spells this way.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Black Cat, Cunning Thief")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Rogue, Subtype::Villain])
        .parse_text(oracle)
        .expect("Black Cat, Cunning Thief should parse strictly");

    assert_eq!(
        unprocessed_compiled_lines(&def).join(" "),
        "When Black Cat enters, look at the top nine cards of target opponent's library, exile two of them face down, then put the rest on the bottom of their library in a random order. You may play the exiled cards for as long as they remain exiled. Mana of any type can be spent to cast spells this way."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_black_cat_cunning_thief_tracks_targets_and_exiled_card_grants() {
    let oracle = "When Black Cat enters, look at the top nine cards of target opponent's library, exile two of them face down, then put the rest on the bottom of their library in a random order. You may play the exiled cards for as long as they remain exiled. Mana of any type can be spent to cast spells this way.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Black Cat, Cunning Thief")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Black Cat, Cunning Thief should parse strictly");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("Target(")
            && debug.contains("Opponent")
            && debug.contains("ChooseObjectsEffect")
            && debug.contains("count: ChoiceCount")
            && debug.contains("min: 2")
            && debug.contains("max: Some")
            && debug.contains("ExileEffect")
            && debug.contains("face_down: true")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect")
            && debug.contains("GrantPlayTaggedEffect")
            && debug.contains("allow_land: true")
            && debug.contains("allow_any_color_for_cast: true"),
        "expected Black Cat target, two-card face-down exile, remainder, and any-mana grants, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_token_with_banding_keyword_modifier() {
    let result = CardDefinitionBuilder::new(CardId::new(), "Errand of Duty Variant")
        .parse_text("Create a 1/1 white Knight creature token with banding.");
    assert!(
        result.is_ok(),
        "token with banding marker should parse as a supported keyword"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_myriad_keyword_as_typed_trigger_without_keyword_marker() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Conclave Evangelist Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Myriad")
        .expect("myriad keyword should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ForPlayersEffect")
            && debug.contains("MayEffect")
            && debug.contains("CreateTokenCopyEffect")
            && !debug.contains("MyriadTokenCopiesEffect"),
        "expected composed myriad trigger effect, got {debug}"
    );
    assert!(
        !debug.contains("StaticAbilityId::KeywordMarker"),
        "myriad should not compile as keyword marker ability: {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_myriad_oracle_text_uses_composed_primitives() {
    let text = "Whenever this creature attacks, for each opponent other than defending player, you may create a token that's a copy of this creature that's tapped and attacking that player or a planeswalker they control. Exile the tokens at end of combat.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Myriad Oracle Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(text)
        .expect("myriad oracle text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreateTokenCopyEffect")
            && debug.contains("exile_at_end_of_combat: true")
            && !debug.contains("MyriadTokenCopiesEffect"),
        "expected composed myriad trigger with exile-at-end-of-combat flag, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wyrms_crossing_patrol_myriad_renders_you_as_token_creator() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wyrm's Crossing Patrol")
        .card_types(vec![CardType::Creature])
        .parse_text("Myriad")
        .expect("wrym's crossing patrol myriad keyword should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("myriad"),
        "expected myriad keyword marker in debug-safe rendering, got {rendered}"
    );
    assert!(
        !rendered.contains("that player may create a token"),
        "myriad render must not flip the token creator to the iterated player, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_frontier_warmonger_trigger_and_menace_grant() {
    let oracle = "Whenever one or more creatures attack one of your opponents or a planeswalker they control, those creatures gain menace until end of turn.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Frontier Warmonger")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Frontier Warmonger should parse strictly");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("one_or_more: true")
            && abilities_debug
                .contains("attacking_player_or_planeswalker_controlled_by: Some(Opponent)")
            && abilities_debug.contains("Some(Menace)"),
        "expected attack trigger with menace grant, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("one or more creature attacking")
            && rendered
                .contains("attacking an opponent or a planeswalker controlled by an opponent")
            && rendered.contains("gains menace until end of turn"),
        "expected compiled text to preserve Frontier Warmonger clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_named_vehicle_token_with_flying_and_crew() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Lita Token Variant")
        .parse_text(
            "{3}{W}, {T}: Create a 5/5 colorless Vehicle artifact token named Zeppelin with flying and crew 3.",
        )
        .expect("named vehicle token should preserve flying and crew");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("name: \"Zeppelin\""),
        "expected created token name to be Zeppelin, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("Flying"),
        "expected created token to keep flying, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CrewCostEffect") && abilities_debug.contains("required_power: 3"),
        "expected created token to keep typed crew ability, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_damage_not_removed_during_cleanup_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ancient Adamantoise Variant")
        .parse_text("Damage isn't removed from this creature during cleanup steps.")
        .expect("damage-not-removed clause should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::DamageNotRemovedDuringCleanup),
        "expected damage-not-removed static ability, got {ids:?}"
    );
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("damage isn't removed from this creature during cleanup steps"),
        "expected compiled text to include damage-not-removed clause, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_damage_redirect_to_source_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Ancient Adamantoise Variant")
        .parse_text("All damage that would be dealt to you and other permanents you control is dealt to this creature instead.")
        .expect("damage redirect clause should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::RedirectDamageToSource),
        "expected damage redirect static ability, got {ids:?}"
    );
    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled
            .to_ascii_lowercase()
            .contains("all damage that would be dealt to you and other permanents you control is dealt to this creature instead"),
        "expected compiled text to include damage redirect clause, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn harsh_judgment_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Harsh Judgment");

    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&StaticAbilityId::ChooseColorAsEnters),
        "expected Harsh Judgment to choose a color as it enters, got {ids:?}"
    );
    assert!(
        ids.contains(&StaticAbilityId::RedirectDamageToSourceController),
        "expected Harsh Judgment damage redirect replacement, got {ids:?}"
    );

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        compiled.contains("As this enchantment enters, choose a color."),
        "expected choose-color clause in compiled text, got {compiled}"
    );
    assert!(
        compiled.contains(
            "If an instant or sorcery spell of the chosen color would deal damage to you, it deals that damage to its controller instead."
        ),
        "expected Harsh Judgment redirect clause in compiled text, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn reverberation_strict_parser_and_compiled_text_regression() {
    let def = parse_oracle_card_definition("Reverberation");

    let compiled = unprocessed_compiled_lines(&def).join(" ");
    assert_eq!(
        compiled,
        "All damage that would be dealt this turn by target sorcery spell is dealt to that spell's controller instead."
    );

    let spell_debug = format!("{:#?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        spell_debug.contains("redirectnexttimedamagetosourceeffect")
            && spell_debug.contains("sorcery")
            && spell_debug.contains("sourcecontroller"),
        "expected Reverberation to compile into a targeted sorcery damage redirect, got {spell_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_red_noncombat_damage_minimum_replacement_line() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Deepest Might Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If a red source you control would deal an amount of noncombat damage less than Deepest Might's power to an opponent, that source deals damage equal to Deepest Might's power instead.",
        )
        .expect("dynamic minimum damage replacement should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::ModifyDamageAmountReplacement),
        "expected minimum damage static ability, got {ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_no_more_than_creatures_can_attack_or_block_each_combat_lines() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Silent Arbiter Variant")
        .parse_text(
            "No more than one creature can attack each combat.\nNo more than one creature can block each combat.",
        )
        .expect("no-more-than attack/block static lines should parse");
    let ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::MaxCreaturesCanAttackEachCombat),
        "expected attack-cap static ability, got {ids:?}"
    );
    assert!(
        ids.contains(&crate::static_abilities::StaticAbilityId::MaxCreaturesCanBlockEachCombat),
        "expected block-cap static ability, got {ids:?}"
    );
    let compiled = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        compiled.contains("no more than 1 creature can attack each combat"),
        "expected compiled text to include attack cap, got {compiled}"
    );
    assert!(
        compiled.contains("no more than 1 creature can block each combat"),
        "expected compiled text to include block cap, got {compiled}"
    );
}
