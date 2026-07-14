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
use super::shard_22::*;
use super::shard_23::*;
use super::*;

#[test]
pub(super) fn parse_oracle_maskwood_nexus_uses_generic_subtype_family_effects() {
    let def = parse_oracle_card_definition("Maskwood Nexus");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();

    assert_eq!(
        static_ids
            .iter()
            .filter(|id| **id == StaticAbilityId::AddAllSubtypesOfFamily)
            .count(),
        3,
        "expected battlefield, stack, and one disjunctive off-battlefield family effect on Maskwood Nexus"
    );
    assert!(
        rendered.contains("creatures you control are every creature type"),
        "expected battlefield clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("creature spells you control are every creature type"),
        "expected stack clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("creature cards in your hand")
            && rendered.contains("creature cards in your library")
            && rendered.contains("creature cards in your graveyard")
            && rendered.contains("creature cards in your exile")
            && rendered.contains("creature cards in your command zone")
            && rendered.contains("every creature type"),
        "expected off-battlefield clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("create a 2/2 blue shapeshifter creature token with changeling"),
        "expected parsed activated token clause, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_neera_wild_mage_consult_cast_bottom() {
    let def = parse_oracle_card_definition("Neera, Wild Mage");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Neera to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.contains("without paying its mana cost"),
        "expected Neera to keep the free-cast follow-up, got {rendered}"
    );
    assert!(
        rendered.contains("bottom of your library"),
        "expected Neera to keep the consult remainder move, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_breaching_dragonstorm_consult_cast_else_hand() {
    let def = parse_oracle_card_definition("Breaching Dragonstorm");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Breaching Dragonstorm to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.contains("without paying its mana cost"),
        "expected Breaching Dragonstorm to keep the free-cast follow-up, got {rendered}"
    );
    assert!(
        rendered.contains("into your hand") || rendered.contains("owner's hand"),
        "expected Breaching Dragonstorm to keep the fallback move to hand, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_beseech_the_mirror_bargain_cast_else_hand() {
    let def = parse_oracle_card_definition("Beseech the Mirror");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("bargain"),
        "expected Beseech the Mirror to keep Bargain, got {rendered}"
    );
    assert!(
        rendered_lower.contains("search your library for a card, exile it face down, then shuffle"),
        "expected Beseech the Mirror to keep the face-down search sequence, got {rendered}"
    );
    assert!(
        rendered_lower.contains("you may cast the exiled card without paying its mana cost if that spell's mana value is 4 or less"),
        "expected Beseech the Mirror to keep the bargained cast gate, got {rendered}"
    );
    assert!(
        rendered_lower.contains("put it into your hand if it wasn't cast this way"),
        "expected Beseech the Mirror to keep the fallback-to-hand clause, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("effect #")
            && !rendered_lower.contains("searched_face_down")
            && !rendered_lower.contains("tagged object"),
        "expected Beseech the Mirror rendering to hide internal scaffolding, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_treasure_keeper_keeps_mana_value_or_less_filter() {
    let def = parse_oracle_card_definition("Treasure Keeper");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("mana value 3 or less"),
        "expected Treasure Keeper to keep its mana-value cap, got {rendered}"
    );
    assert!(
        !rendered.contains("named less"),
        "expected Treasure Keeper to avoid bogus named-Less parsing, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_glamdring_keeps_damage_scaled_free_cast_clause() {
    let def = parse_oracle_card_definition("Glamdring");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Glamdring to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.contains("cast an instant or sorcery spell")
            && rendered.contains("from your hand")
            && rendered.contains("without paying its mana cost"),
        "expected Glamdring to keep its hand free-cast clause, got {rendered}"
    );
    assert!(
        rendered.contains("first strike"),
        "expected Glamdring to keep granted first strike, got {rendered}"
    );
    assert!(
        rendered.contains("mana value less than or equal to that amount"),
        "expected Glamdring to keep the dynamic damage-based mana value limit, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_transmogrify_shuffle_rest_into_library() {
    let def = parse_oracle_card_definition("Transmogrify");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Transmogrify to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.contains("shuffle"),
        "expected Transmogrify to keep its shuffle remainder, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_transmogrify_keeps_iterated_library_owner() {
    let def = parse_oracle_card_definition("Transmogrify");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("their library"),
        "expected Transmogrify to keep the target controller's library, got {rendered}"
    );
    assert!(
        rendered.contains("onto the battlefield"),
        "expected Transmogrify to keep its battlefield hit, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_hurl_into_history_discovers_that_spells_mana_value() {
    let def = parse_oracle_card_definition("Hurl into History");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Hurl into History to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.contains("discover"),
        "expected Hurl into History to keep discover, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_monstrous_vortex_discovers_that_spells_mana_value() {
    let def = parse_oracle_card_definition("Monstrous Vortex");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Monstrous Vortex to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.contains("discover"),
        "expected Monstrous Vortex to keep discover, got {rendered}"
    );
}

#[test]
pub(super) fn parse_oracle_curator_of_suns_creation_discovers_same_value_again() {
    let def = parse_oracle_card_definition("Curator of Sun's Creation");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        !rendered.contains("unsupported effect"),
        "expected Curator of Sun's Creation to compile without unsupported effects, got {rendered}"
    );
    assert!(
        rendered.matches("discover").count() >= 2,
        "expected Curator of Sun's Creation to keep both discover actions, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_planar_genesis_looked_card_fallback_sequence() {
    CardDefinitionBuilder::new(CardId::new(), "Planar Genesis Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Look at the top four cards of your library. You may put a land card from among them onto the battlefield tapped. If you don't, put a card from among them into your hand. Put the rest on the bottom of your library in a random order.",
        )
        .expect("looked-card battlefield-or-hand fallback should parse");
}

#[test]
pub(super) fn parse_oracle_bounty_of_skemfar_split_reveal_selection_regression() {
    let def = parse_oracle_card_definition("Bounty of Skemfar");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "reveal the top six cards of your library. you may put up to one land card from among them onto the battlefield tapped and up to one elf card from among them into your hand. put the rest on the bottom of your library in a random order"
        ),
        "expected oracle-shaped bounty text, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("PutTaggedRemainderOnLibraryBottomEffect")
            && debug.contains("LookAtTopCardsEffect")
            && debug.matches("ChooseObjectsEffect").count() >= 2
            && debug.contains("PutOntoBattlefieldEffect")
            && debug.contains("MoveToZoneEffect")
            && debug.contains("IsTaggedObject")
            && debug.contains("IsNotTaggedObject"),
        "expected structured looked-card split-choice lowering, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_harper_recruiter_repeated_subtype_reveal_regression() {
    let def = parse_oracle_card_definition("Harper Recruiter");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("look at the top four cards of your library")
            && rendered_lower.contains(
                "you may reveal a cleric card, a rogue card, a warrior card, and/or a wizard card from among them and put those cards into your hand"
            )
            && rendered_lower
                .contains("put the rest on the bottom of your library in a random order"),
        "expected oracle-shaped Harper Recruiter looked-card text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LookAtTopCardsEffect")
            && debug.matches("ChooseObjectsEffect").count() >= 4
            && debug.contains("RevealTaggedEffect")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected structured repeated subtype looked-card lowering, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_selective_adaptation_keyword_bundle_regression() {
    let def = parse_oracle_card_definition("Selective Adaptation");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        !rendered_lower.contains("another permanent"),
        "expected Selective Adaptation to stop misparsing as a bounce effect, got {rendered}"
    );
    // Honest surface: the keyword-choice bundle renders as a chain of choose
    // sentences (the compact "choose from among them ..." wording was a
    // deleted hand-written gate).  FIXME(render): compact bundle rendering.
    assert!(
        rendered_lower.contains("choose up to one other cards with flying")
            && rendered_lower.contains("choose up to one other cards with first strike"),
        "expected Selective Adaptation to preserve its keyword-choice bundle, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.matches("ChooseObjectsEffect").count() >= 13,
        "expected repeated keyword picks plus battlefield choice, got {debug}"
    );
    assert!(
        debug.contains("zone: Hand")
            && debug.contains("zone: Graveyard")
            && debug.contains("zone: Battlefield"),
        "expected battlefield, hand, and graveyard moves in Selective Adaptation, got {debug}"
    );
    assert!(
        debug.contains("Flying")
            && debug.contains("FirstStrike")
            && debug.contains("DoubleStrike")
            && debug.contains("Vigilance"),
        "expected keyword-specific filters in Selective Adaptation, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_akroma_vision_of_ixidor_keyword_bundle_regression() {
    let def = parse_oracle_card_definition("Akroma, Vision of Ixidor");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("at the beginning of each combat")
            && rendered_lower.contains("until end of turn"),
        "expected Akroma trigger timing to remain intact, got {rendered}"
    );
    assert!(
        rendered_lower.contains("other creatures with flying you control get +1/+1")
            && rendered_lower.contains("other creatures with vigilance you control get +1/+1"),
        "expected Akroma compiled text to keep the oracle-shaped keyword bundle, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("unsupported"),
        "expected Akroma to compile without fallback placeholder text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeginningOfCombat")
            && debug.matches("ApplyContinuousEffect").count() >= 14
            && debug.matches("ModifyPowerToughness").count() >= 14,
        "expected Akroma trigger to lower into repeated keyword-based pump effects, got {debug}"
    );
    assert!(
        debug.contains("Flying")
            && debug.contains("FirstStrike")
            && debug.contains("DoubleStrike")
            && debug.contains("Deathtouch")
            && debug.contains("Haste")
            && debug.contains("Hexproof")
            && debug.contains("Indestructible")
            && debug.contains("Lifelink")
            && debug.contains("Menace")
            && debug.contains("Protection")
            && debug.contains("Reach")
            && debug.contains("Trample")
            && debug.contains("Vigilance")
            && debug.contains("Partner"),
        "expected Akroma trigger filters for the full keyword bundle, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_winding_way_card_type_choice_regression() {
    let def = parse_oracle_card_definition("Winding Way");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "choose creature or land. reveal the top four cards of your library. put all cards of the chosen type revealed this way into your hand and the rest into your graveyard."
        ),
        "expected oracle-shaped winding way text, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseCardTypeEffect")
            && debug.contains("LookAtTopCardsEffect")
            && debug.contains("zone: Graveyard"),
        "expected chosen-card-type looked-card lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_top_put_all_matching_into_hand_rest_graveyard_still_handles_simple_filters()
 {
    let def = CardDefinitionBuilder::new(CardId::new(), "Simple Reveal Split Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal the top five cards of your library. Put all creature cards revealed this way into your hand and the rest into your graveyard.",
        )
        .expect("simple reveal-top split should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reveal the top five cards of your library")
            && rendered.contains("put all creature cards revealed this way into your hand and the rest into your graveyard"),
        "expected simple reveal-top split to stay intact, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_reveal_top_may_put_matching_card_into_hand_rest_graveyard_regression() {
    let def = CardDefinitionBuilder::new(CardId::new(), "May Reveal Split Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Reveal the top five cards of your library. You may put a creature or enchantment card from among them into your hand. Put the rest into your graveyard.",
        )
        .expect("may reveal-top split should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("reveal the top five cards of your library")
            && rendered_lower.contains(
                "you may put a creature or enchantment card from among them into your hand"
            )
            && rendered_lower.contains("put the rest into your graveyard"),
        "expected may reveal-top split to stay oracle-shaped, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(Library)")
            && !debug.contains("additional_zones: [Hand"),
        "expected looked-card choice to stay scoped to library, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacrifice_reveal_top_split_chosen_lands_and_nonlands_regression() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Hew the Entwood Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Sacrifice any number of lands. Reveal the top X cards of your library, where X is the number of lands sacrificed this way. Choose any number of artifact and/or land cards revealed this way. Put all nonland cards chosen this way onto the battlefield, then put all land cards chosen this way onto the battlefield tapped, then put the rest on the bottom of your library in a random order.",
        )
        .expect("sacrifice/reveal/split/remainder sequence should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("sacrifice any number of lands")
            && rendered_lower.contains(
                "reveal the top x cards of your library, where x is the number of lands sacrificed this way"
            )
            && rendered_lower.contains(
                "choose any number of artifact and/or land cards revealed this way"
            )
            && rendered_lower.contains("put all nonland cards chosen this way onto the battlefield")
            && rendered_lower.contains(
                "put all land cards chosen this way onto the battlefield tapped"
            )
            && rendered_lower.contains(
                "put the rest on the bottom of your library in a random order"
            ),
        "expected Hew-style composed sequence to render oracle-shaped, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("LookAtTopCardsEffect")
            && debug.contains("ForEachTaggedEffect")
            && debug.contains("ConditionalEffect")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect")
            && (debug.contains("EffectMetric") || debug.contains("EffectValue")),
        "expected typed sacrifice/reveal/choice/split/remainder effects, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_hurkyl_master_wizard_card_type_gather_regression() {
    let def = parse_oracle_card_definition("Hurkyl, Master Wizard");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("reveal the top five cards of your library")
            || rendered_lower.contains("look at the top five cards of your library"),
        "expected Hurkyl to keep the top-five reveal, got {rendered}"
    );
    assert!(
        !rendered.contains("__sentence_helper"),
        "expected Hurkyl to avoid leaking helper tags, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LookAtTopCardsEffect")
            && debug.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected looked-card gather lowering for Hurkyl, got {debug}"
    );
    assert!(
        !debug.contains("RevealTopEffect") && !debug.contains("ForEachObject"),
        "expected Hurkyl to avoid the old reveal-top/foreach-object lowering, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_bend_or_break_keeps_divvy_pile_control_binding() {
    let def = parse_oracle_card_definition("Bend or Break");
    let rendered = crate::compiled_text::canonical_compiled_lines(&def).join(" ");

    assert_eq!(
        rendered,
        "Each player separates all nontoken lands they control into two piles. For each player, one of their piles is chosen by one of their opponents of their choice. Destroy all lands in the chosen piles. Tap all lands in the other piles."
    );

    let raw = format!("{:?}", def.spell_effect);
    assert!(
        raw.contains("chooser: TaggedPlayer") && raw.contains("controller: Some(IteratedPlayer)"),
        "expected chosen opponent to choose from the iterated player's lands, got {raw}"
    );
}

#[test]
pub(super) fn parse_oracle_turnabout_card_type_mass_tap_choice_regression() {
    let def = parse_oracle_card_definition("Turnabout");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("choose artifact, creature, or land"),
        "expected explicit card-type choice, got {rendered}"
    );
    assert!(
        rendered_lower.contains("tap all")
            && rendered_lower.contains("untap all")
            && rendered_lower.contains("chosen type"),
        "expected tap/untap mass-action text to keep the chosen-type reference, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseCardTypeEffect")
            && debug.contains("ChooseModeEffect")
            && debug.contains("TapEffect")
            && debug.contains("UntapEffect")
            && !debug.contains("UnlessActionEffect"),
        "expected modal mass tap/untap lowering without unless fallback, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_teferis_realm_type_choice_phase_out_regression() {
    let def = parse_oracle_card_definition("Teferi's Realm");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("all nontoken permanents of that type phase out"),
        "expected the phase-out line to stay present, got {rendered}"
    );

    let debug = format!("{:#?}", def);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("PhaseOutEffect")
            && debug.contains("SharesCardType")
            && debug.contains("Aura"),
        "expected Teferi's Realm to keep a type-linked choose/phase-out bundle, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_creeping_renaissance_permanent_type_choice_regression() {
    let def = parse_oracle_card_definition("Creeping Renaissance");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("choose a permanent type"),
        "expected permanent-type choice wording, got {rendered}"
    );
    assert!(
        rendered_lower
            .contains("return all cards of the chosen type from your graveyard to your hand"),
        "expected chosen-type graveyard return wording, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect).to_ascii_lowercase();
    assert!(
        debug.contains("choosecardtypeeffect"),
        "expected choose-card-type lowering, got {debug}"
    );
    assert!(
        !debug.contains("choosecreaturetypeeffect"),
        "creeping renaissance should not lower a creature-type choice, got {debug}"
    );
    assert!(
        debug.contains("returntohandeffect") && debug.contains("spec: all("),
        "expected all-cards return-to-hand lowering, got {debug}"
    );
    assert!(
        debug.contains("chosen_creature_type: true"),
        "expected chosen-type graveyard filter, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_owlbear_shepherd_total_power_intervening_if_regression() {
    let def = parse_oracle_card_definition("Owlbear Shepherd");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "at the beginning of your end step, if creatures you control have total power 8 or greater, draw a card"
        ),
        "expected total-power intervening-if trigger text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ControlCreaturesTotalPowerAtLeast(8)"),
        "expected total-power condition lowering, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_opaline_bracers_charge_counter_scaling_regression() {
    let def = parse_oracle_card_definition("Opaline Bracers");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "equipped creature gets +x/+x, where x is the number of charge counters on this"
        ) && !rendered_lower.contains("for each equipment"),
        "expected counter-based equipment scaling text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CountersOnSource(Charge)")
            || (debug.contains("CountersOnSourceWithSurface")
                && debug.contains("counter_type: Charge")),
        "expected anthem scaling to count charge counters on the source, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_magma_mine_pressure_counter_damage_regression() {
    let def = parse_oracle_card_definition("Magma Mine");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("deals damage")
            && rendered.contains("to any target")
            && rendered.contains("equal to the number of pressure counters on this artifact"),
        "expected pressure-counter damage wording, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CountersOnSource(Named(\"pressure\"))"),
        "expected activated ability to count pressure counters on source, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_commanders_insignia_commander_cast_count_regression() {
    let def = parse_oracle_card_definition("Commander's Insignia");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "creatures you control get +1/+1 for each time you've cast your commander from the command zone this game"
        ),
        "expected commander-cast-count anthem wording, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CommanderCastCount(You)"),
        "expected commander-cast-count lowering in parsed ability, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_study_hall_commander_cast_scry_regression() {
    let def = parse_oracle_card_definition("Study Hall");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("scry x")
            && rendered.contains(
                "where x is the number of times you've cast your commander from the command zone this game"
            ),
        "expected Study Hall to render commander-cast-count scry text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CommanderCastCount(You)"),
        "expected Study Hall trigger to lower x as your commander cast count, got {debug}"
    );
    assert!(
        debug.contains("SpellCastTrigger")
            && debug.contains("is_commander: true")
            && debug.contains("owner: Some(You)")
            && debug.contains("caster: You"),
        "expected Study Hall trigger to stay scoped to casting your commander, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_emissary_escort_greatest_mana_value_anthem_regression() {
    let def = parse_oracle_card_definition("Emissary Escort");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("greatest mana value among other artifacts you control"),
        "expected greatest-mana-value anthem wording, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("GreatestManaValueAmong"),
        "expected greatest-mana-value anthem lowering in parsed ability, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_clarion_ultimatum_for_each_chosen_permanent_regression() {
    let def = parse_oracle_card_definition("Clarion Ultimatum");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains("choose")
            && rendered_lower.contains("search your library")
            && rendered_lower.contains("same name")
            && rendered_lower.contains("onto the battlefield tapped")
            && rendered_lower.contains("shuffle"),
        "expected clarion ultimatum text to keep choose/search/battlefield/shuffle sequence, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ForEachTaggedEffect") && debug.contains("SameNameAsTagged"),
        "expected same-name search to iterate once per chosen permanent, got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_riveteers_charm_strict_regression() {
    let def = parse_oracle_card_definition("Riveteers Charm");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("greatest mana value among creatures and planeswalkers they control"),
        "expected greatest-mana-value sacrifice clause in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("until your next end step, you may play those cards"),
        "expected next-end-step play duration in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains("exile target player's graveyard"),
        "expected graveyard-exile mode in compiled text, got {rendered}"
    );

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("SacrificePlayerEffect") && debug.contains("GreatestManaValue"),
        "expected sacrifice effect constrained by greatest mana value, got {debug}"
    );
    assert!(
        debug.contains("UntilYourNextTurnEnd"),
        "expected timing branch for 'until your next end step', got {debug}"
    );
}

#[test]
pub(super) fn parse_oracle_rakdos_the_muscle_strict_and_compiled_text_regression() {
    assert_oracle_card_parses_strict("Rakdos, the Muscle");
    let def = parse_oracle_card_definition("Rakdos, the Muscle");
    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();

    assert!(
        rendered_lower.contains(
            "whenever you sacrifice another creature, exile cards equal to its mana value from the top of target player's library"
        ),
        "expected dynamic sacrificed-creature mana-value exile clause, got {rendered}"
    );
    assert!(
        rendered_lower.contains(
            "until your next end step, you may play those cards, and mana of any type can be spent to cast those spells"
        ),
        "expected next-end-step play permission with any-color mana suffix, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("ExileTopOfLibraryEffect")
            && debug.contains("ManaValueOf")
            && debug.contains("GrantPlayTaggedEffect")
            && debug.contains("UntilYourNextTurnEnd")
            && debug.contains("allow_any_color_for_cast: true"),
        "expected Rakdos trigger to lower to dynamic top-library exile plus next-end-step any-color play grant, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn planar_genesis_fallback_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Planar Genesis Variant")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Look at the top four cards of your library. You may put a land card from among them onto the battlefield tapped. If you don't, put a card from among them into your hand this turn. Put the rest on the bottom of your library in a random order.",
        )
        .expect_err("unsupported looked-card fallback tail should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not parse")
            || rendered.contains("expected"),
        "expected loud failure for unsupported looked-card fallback tail, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_caustic_bronco_saddled_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Caustic Bronco Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, reveal the top card of your library and put it into your hand. You lose life equal to that card's mana value if this creature isn't saddled. Otherwise, each opponent loses that much life.\nSaddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)",
        )
        .expect("saddled conditional reveal-life trigger should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn caustic_bronco_saddled_followup_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Caustic Bronco Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, reveal the top card of your library and put it into your hand. You lose life equal to that card's mana value if this creature isn't saddled this turn. Otherwise, each opponent loses that much life.\nSaddle 3 (Tap any number of other creatures you control with total power 3 or more: This Mount becomes saddled until end of turn. Saddle only as a sorcery.)",
        )
        .expect_err("unsupported saddled conditional tail should fail");
    let rendered = err.to_string();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("could not parse")
            || rendered.contains("expected"),
        "expected loud failure for unsupported saddled conditional tail, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_minsc_and_boo_hamster_followup_condition() {
    CardDefinitionBuilder::new(CardId::new(), "Minsc Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "When this permanent enters and at the beginning of your upkeep, you may create Boo, a legendary 1/1 red Hamster creature token with trample and haste.\n+1: Put three +1/+1 counters on up to one target creature with trample or haste.\n-2: Sacrifice a creature. When you do, this permanent deals X damage to any target, where X is that creature's power. If the sacrificed creature was a Hamster, draw X cards.",
        )
        .expect("sacrificed-creature subtype followup should parse");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn sacrificed_creature_was_hamster_with_extra_tail_still_fails_loudly() {
    let err = CardDefinitionBuilder::new(CardId::new(), "Hamster Tail Negative Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text(
            "-2: Sacrifice a creature. When you do, this permanent deals X damage to any target, where X is that creature's power. If the sacrificed creature was a Hamster this turn, draw X cards.",
        )
        .expect_err("unsupported sacrificed-creature predicate tail should fail");

    let rendered = format!("{err:?}").to_ascii_lowercase();
    assert!(
        rendered.contains("unsupported")
            || rendered.contains("unsupported predicate")
            || rendered.contains("could not find verb"),
        "expected loud failure for unsupported sacrificed-creature tail, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_can_be_your_commander_static_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Dihada Variant")
        .card_types(vec![CardType::Planeswalker])
        .parse_text("This can be your commander.")
        .expect("can-be-commander line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::CanBeCommander),
        "expected can-be-commander static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_library_reveal_disjunction_to_hand_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Archdruid's Charm Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Search your library for a creature or land card, reveal it, put it into your hand, then shuffle.")
        .expect("search reveal disjunction to hand should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        (debug.contains("SearchLibraryEffect") || debug.contains("ChooseObjectsEffect"))
            && debug.contains("chooser: You")
            && (debug.contains("destination: Hand") || debug.contains("zone: Hand")),
        "expected search-library hand effect with explicit chooser, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_library_face_down_exile_then_shuffle_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Hoarding Broodlord Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, search your library for a card, exile it face down, then shuffle.")
        .expect("search face-down exile clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("ExileEffect")
            && debug.contains("face_down: true")
            && debug.contains("ShuffleLibraryEffect"),
        "expected choose-plus-face-down-exile search sequence, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("exile it face down")
            && !rendered.contains("searched_face_down")
            && !rendered.contains("tagged object"),
        "expected face-down search rendering to hide the internal tag, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_inverter_of_truth_etb_clause_keeps_face_down_library_exile() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Inverter of Truth Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, exile all cards from your library face down, then shuffle all cards from your graveyard into your library.")
        .expect("Inverter of Truth-style ETB clause should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ExileEffect")
            && debug.contains("face_down: true")
            && debug.contains("ShuffleGraveyardIntoLibraryEffect"),
        "expected face-down library exile plus graveyard shuffle, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile all cards from your library face down")
            && rendered.contains("shuffle all cards from your graveyard into your library"),
        "expected inverter-style oracle text to stay intact, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tithe_separate_searches_then_reveal_to_hand() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Tithe Variant")
        .card_types(vec![CardType::Instant])
        .parse_text("Search your library for a Plains card. If an opponent controls more lands than you, you may search your library for an additional Plains card. Reveal those cards. Put them into your hand. Then shuffle.")
        .expect("Tithe-style repeated search should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("RevealTaggedEffect")
            && debug.contains("MoveToZone")
            && debug.contains("zone: Hand")
            && debug.contains("ShuffleLibraryEffect"),
        "expected repeated tagged search, reveal, hand move, and shuffle, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oreskos_explorer_uses_player_land_comparison_for_x() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Oreskos Explorer")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .parse_text(
            "When this creature enters, search your library for up to X Plains cards, where X is the number of players who control more lands than you. Reveal those cards, put them into your hand, then shuffle.",
        )
        .expect("Oreskos Explorer text should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("up_to_x: true")
            && debug.contains("PlayersWhoControlMoreThanYou")
            && debug.contains("count_value: Some"),
        "expected Oreskos Explorer to preserve the optional dynamic count and player-comparison value, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("up to x plains")
            && rendered.contains("players who control more lands than you")
            && rendered.contains("reveal those cards")
            && rendered.contains("put them into your hand")
            && rendered.contains("shuffle"),
        "expected Oreskos Explorer oracle-like text to stay close to the card, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oath_of_druids_maps_to_upkeep_consult_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Oath of Druids")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.",
        )
        .expect("Oath of Druids should parse into its consult trigger");

    let raw = format!("{:?}", def.abilities);
    assert!(
        raw.contains("BeginningOfUpkeepTrigger")
            && raw.contains("player: Any")
            && raw.contains("AnOpponentControlsMoreThanPlayer")
            && raw.contains("ConsultTopOfLibraryEffect")
            && raw.contains("MayEffect")
            && raw.contains("zone: Battlefield")
            && raw.contains("zone: Graveyard"),
        "expected Oath of Druids to keep its upkeep consult structure, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player's upkeep")
            && (rendered.contains("an opponent controls more creatures than that player")
                || rendered.contains("controls more creatures than they do and is their opponent"))
            && (rendered.contains("that player may reveal cards from the top of")
                || rendered.contains("the first player may reveal cards from the top of"))
            && rendered.contains("until they reveal a creature card")
            && rendered.contains("that card onto the battlefield")
            && rendered.contains("all other cards revealed this way into their graveyard"),
        "expected Oath of Druids oracle-like text to stay close to the oracle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oath_of_ghouls_maps_to_upkeep_return_effect() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Oath of Ghouls")
        .card_types(vec![CardType::Enchantment])
        .parse_text(
            "At the beginning of each player's upkeep, that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent. The first player may return a creature card from their graveyard to their hand.",
        )
        .expect("Oath of Ghouls should parse into its upkeep return trigger");

    let raw = format!("{:?}", def.abilities);
    assert!(
        raw.contains("BeginningOfUpkeepTrigger")
            && raw.contains("player: Any")
            && raw.contains("AnOpponentHasFewerThanPlayer")
            && raw.contains("MayEffect")
            && raw.contains("ReturnFromGraveyardToHandEffect")
            && raw.contains("player: Active")
            && raw.contains("owner: Some(Active)"),
        "expected Oath of Ghouls to keep its upkeep graveyard-return structure, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each player's upkeep")
            && rendered
                .contains("that player chooses target player whose graveyard has fewer creature cards in it than their graveyard does and is their opponent")
            && rendered.contains("the first player may return a creature card from their graveyard to their hand")
            && !rendered.contains("active player's graveyard"),
        "expected Oath of Ghouls oracle-like text to stay close to the oracle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_mind_funeral_tracks_passive_consult_count_and_graveyard_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Mind Funeral")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent reveals cards from the top of their library until four land cards are revealed. That player puts all cards revealed this way into their graveyard.",
        )
        .expect("Mind Funeral should parse");

    let spell_effect = def.spell_effect.as_ref().expect("spell effects");
    assert_eq!(spell_effect.segments.len(), 1);
    let effects = &spell_effect.segments[0].default_effects;
    let graveyard_move_is_ok = |effect: &Effect| {
        effect
            .downcast_ref::<MoveToZoneEffect>()
            .is_some_and(|move_to_zone| {
                move_to_zone.zone == Zone::Graveyard
                    && matches!(&move_to_zone.target, ChooseSpec::Tagged(_))
            })
            || effect.downcast_ref::<TaggedEffect>().is_some_and(|effect| {
                effect
                    .effect
                    .downcast_ref::<MoveToZoneEffect>()
                    .is_some_and(|move_to_zone| {
                        move_to_zone.zone == Zone::Graveyard
                            && matches!(&move_to_zone.target, ChooseSpec::Tagged(_))
                    })
            })
    };
    assert!(
        effects.len() == 3
            && effects[0].downcast_ref::<TargetOnlyEffect>().is_some()
            && effects[1]
                .downcast_ref::<ConsultTopOfLibraryEffect>()
                .is_some_and(|effect| {
                    effect.mode == crate::effects::consult_helpers::LibraryConsultMode::Reveal
                        && matches!(
                            &effect.stop_rule,
                            crate::effects::ConsultTopOfLibraryStopRule::MatchCount(
                                crate::effect::Value::Fixed(4)
                            )
                        )
                })
            && graveyard_move_is_ok(&effects[2]),
        "expected Mind Funeral to lower to a target-only consult plus tagged graveyard move, got {spell_effect:?}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("all cards revealed this way into")
            && !rendered.contains("put it into its owner's graveyard"),
        "expected Mind Funeral compiled text to use the plural revealed-set wording, got {rendered}"
    );

    let oracle_rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        (oracle_rendered.contains("until four land cards are revealed")
            || oracle_rendered.contains("until they reveal 4 land cards"))
            && oracle_rendered.contains("all cards revealed this way into"),
        "expected Mind Funeral oracle-like text to stay close to the oracle, got {oracle_rendered}"
    );
}

#[test]
pub(super) fn parse_corpse_appraiser_keeps_the_exile_then_loot_sequence() {
    let def = parse_oracle_card_definition("Corpse Appraiser");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("Exile")
            && (abilities_debug.contains("IfResult") || abilities_debug.contains("IfEffect"))
            && abilities_debug.contains("LookAtTopCardsEffect")
            && abilities_debug.contains("zone: Hand")
            && abilities_debug.contains("zone: Graveyard"),
        "expected Corpse Appraiser to keep exile plus the hand-and-graveyard looked-card split, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile up to one target creature card")
            && (rendered.contains("if a card is put into exile this way")
                || rendered.contains("if you do"))
            && rendered.contains("look at the top three cards of your library")
            && rendered
                .contains("put one of those cards into your hand and the rest into your graveyard")
            && !rendered.contains("put it into its owner's hand"),
        "expected Corpse Appraiser compiled text to preserve the looked-card split, got {rendered}"
    );
}

#[test]
pub(super) fn parse_uchuulon_keeps_if_you_do_exile_followup() {
    let def = parse_oracle_card_definition("Uchuulon");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("if you do, create a token that's a copy of this creature")
            && !rendered.contains("if a card is put into exile this way"),
        "expected Uchuulon to keep its oracle-style if-you-do exile followup, got {rendered}"
    );
}

#[test]
pub(super) fn parse_nyla_shirshu_sleuth_keeps_if_you_do_exile_followup() {
    let def = parse_oracle_card_definition("Nyla, Shirshu Sleuth");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        (rendered.contains("if you do, you lose life equal to its mana value")
            || rendered.contains("if you do, you lose x life and create x clue tokens, where x is that card's mana value"))
            && !rendered.contains("if a card is put into exile this way"),
        "expected Nyla to keep its oracle-style if-you-do exile followup, got {rendered}"
    );
}

#[test]
pub(super) fn parse_thief_of_existence_keeps_if_you_do_exile_followup() {
    let def = parse_oracle_card_definition("Thief of Existence");
    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();

    assert!(
        rendered.contains("if you do, this creature gains")
            && !rendered.contains("if a card is put into exile this way"),
        "expected Thief of Existence to keep its oracle-style if-you-do exile followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_demonic_consultation_renders_chosen_name_consult() {
    let def = parse_oracle_card_definition("Demonic Consultation");
    let effects_debug = format!("{:?}", def.spell_effect);
    assert!(
        effects_debug.contains("ChooseCardNameEffect")
            && effects_debug.contains("ExileTopOfLibraryEffect")
            && effects_debug.contains("ConsultTopOfLibraryEffect")
            && effects_debug.contains("MoveToZoneEffect")
            && effects_debug.contains("zone: Exile"),
        "expected Demonic Consultation to lower to chosen-name consult, hand move, and exile remainder, got {effects_debug}"
    );

    let spell_effect = def
        .spell_effect
        .as_ref()
        .expect("Demonic Consultation should have a spell effect");
    let direct_rendered =
        crate::compiled_text::compile_effect_list(&spell_effect.segments[0].default_effects);
    assert!(
        direct_rendered
            .to_ascii_lowercase()
            .contains("until you reveal a card with the chosen name"),
        "expected direct Demonic Consultation effect rendering to compact chosen-name consult, got {direct_rendered}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("choose a card name")
            && rendered_lower.contains("exile the top six cards of your library")
            && rendered_lower.contains("until you reveal a card with the chosen name")
            && rendered_lower.contains("put that card into your hand")
            && rendered_lower.contains("exile all other cards revealed this way"),
        "expected Demonic Consultation to render compact chosen-name consult text, got {rendered}"
    );
    assert!(
        !rendered_lower.contains("same name as that object")
            && !rendered_lower.contains("unless it's a permanent"),
        "expected Demonic Consultation rendering to avoid generic fallback text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_daring_waverider_renders_limited_free_graveyard_cast_replacement() {
    let def = parse_oracle_card_definition("Daring Waverider");
    let effects_debug = format!("{:?}", def.abilities);
    assert!(
        effects_debug.contains("LessThanOrEqual(4)")
            && effects_debug.contains("without_paying_mana_cost: true")
            && effects_debug.contains("RegisterFutureZoneReplacementEffect"),
        "expected Daring Waverider to keep mana-value-limited free-cast replacement, got {effects_debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("When this creature enters, you may cast target instant or sorcery card with mana value 4 or less from your graveyard without paying its mana cost. If that spell would be put into your graveyard, exile it instead"),
        "expected Daring Waverider to render compact free-cast replacement text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tourachs_canticle_renders_reveal_choose_discard_sequence() {
    let def = parse_oracle_card_definition("Tourach's Canticle");
    let effects_debug = format!("{:?}", def.spell_effect);
    assert!(
        effects_debug.contains("LookAtHandEffect")
            && effects_debug.contains("ChooseObjectsEffect")
            && effects_debug.contains("DiscardEffect")
            && effects_debug.contains("random: true"),
        "expected Tourach's Canticle to lower to reveal, choose, discard chosen, and random discard, got {effects_debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Target opponent reveals their hand")
            && rendered.contains("You choose a card from it")
            && rendered.contains("That player discards that card, then discards a card at random"),
        "expected Tourach's Canticle to render compact reveal/choose/discard text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_duneblast_renders_choose_up_to_one_destroy_rest() {
    let def = parse_oracle_card_definition("Duneblast");
    let effects_debug = format!("{:?}", def.spell_effect);
    assert!(
        effects_debug.contains("ChooseObjectsEffect")
            && effects_debug.contains("DestroyEffect")
            && effects_debug.contains("IsNotTaggedObject"),
        "expected Duneblast to lower to choose one creature and destroy nonchosen creatures, got {effects_debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Choose up to one creature. Destroy the rest"),
        "expected Duneblast to render choose/destroy-rest text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_phyrexian_war_beast_renders_sacrifice_land_damage() {
    let def = parse_oracle_card_definition("Phyrexian War Beast");
    let effects_debug = format!("{:?}", def.abilities);
    assert!(
        effects_debug.contains("ChooseObjectsEffect")
            && effects_debug.contains("SacrificePlayerEffect")
            && effects_debug.contains("DealDamageEffect")
            && effects_debug.contains("SourceController"),
        "expected Phyrexian War Beast to lower to choose/sacrifice land and source-controller damage, got {effects_debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("sacrifice a land")
            && rendered_lower.contains("this creature deals 1 damage to you"),
        "expected Phyrexian War Beast to render sacrifice-land damage text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacred_guide_uses_consult_white_card_lowering() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sacred Guide")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{W}, Sacrifice this creature: Reveal cards from the top of your library until you reveal a white card. Put that card into your hand and exile all other cards revealed this way.",
        )
        .expect("Sacred Guide should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ConsultTopOfLibraryEffect")
            && abilities_debug.contains("MoveToZoneEffect")
            && abilities_debug.contains("zone: Exile")
            && abilities_debug.contains("SacrificeTargetEffect"),
        "expected Sacred Guide to lower to consult, hand move, exile remainder, and sacrifice cost, got {abilities_debug}"
    );
    assert!(
        !abilities_debug.contains("RevealTopEffect"),
        "expected Sacred Guide to avoid the generic reveal-top fallback, got {abilities_debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower
            .contains("reveal cards from the top of your library until you reveal a white card")
            && rendered_lower.contains("put that card into your hand")
            && rendered_lower.contains("exile all other cards revealed this way"),
        "expected Sacred Guide compiled text to preserve the consult-and-exile wording, got {rendered}"
    );
    assert!(
        !rendered.to_ascii_lowercase().contains("another permanents"),
        "expected Sacred Guide compiled text to avoid the generic reveal fallback wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sacred_guide_reveals_until_white_card_and_exiles_others() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sacred Guide")
        .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Human, Subtype::Cleric])
        .power_toughness(PowerToughness::fixed(1, 1))
        .parse_text(
            "{1}{W}, Sacrifice this creature: Reveal cards from the top of your library until you reveal a white card. Put that card into your hand and exile all other cards revealed this way.",
        )
        .expect("Sacred Guide should parse");

    let ability = def
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => Some(activated),
            _ => None,
        })
        .expect("Sacred Guide should have an activated ability");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let guide_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(2), "Bottom Card")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(3), "White Hit")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );
    game.create_object_from_card(
        &crate::card::CardBuilder::new(CardId::from_raw(4), "Blue Miss")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Creature])
            .build(),
        alice,
        Zone::Library,
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(guide_id, alice, &mut dm);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        guide_id,
        &ability.effects,
        None,
        &[],
    )
    .expect("Sacred Guide effect should resolve");

    let hand_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .hand
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        hand_names.iter().any(|name| name == "White Hit"),
        "Sacred Guide should put the first white card into hand, got {hand_names:?}"
    );
    assert!(
        !hand_names.iter().any(|name| name == "Blue Miss"),
        "Sacred Guide should not put nonmatching cards into hand, got {hand_names:?}"
    );

    let exile_names: Vec<_> = game
        .exile
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert!(
        exile_names.iter().any(|name| name == "Blue Miss"),
        "Sacred Guide should exile the revealed nonwhite card, got {exile_names:?}"
    );
    assert!(
        !exile_names.iter().any(|name| name == "White Hit"),
        "Sacred Guide should keep the matching white card out of exile, got {exile_names:?}"
    );

    let library_names: Vec<_> = game
        .player(alice)
        .expect("alice exists")
        .library
        .iter()
        .filter_map(|&id| game.object(id).map(|obj| obj.name.to_string()))
        .collect();
    assert_eq!(
        library_names,
        vec!["Bottom Card".to_string()],
        "Sacred Guide should stop at the first white card and leave the unseen library cards alone"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_master_warcraft_uses_combat_choice_control_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Master Warcraft")
        .parse_text(
            "Cast this spell only before attackers are declared.\nYou choose which creatures attack this turn.\nYou choose which creatures block this turn and how those creatures block.",
        )
        .expect("Master Warcraft should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ControlCombatChoicesThisTurnEffect"),
        "expected Master Warcraft to lower to combat-choice control effects, got {spell_debug}"
    );

    let rendered = crate::compiled_text::unprocessed_compiled_lines(&def).join("\n");
    assert_eq!(
        rendered,
        "Cast this spell only before attackers are declared.\nYou choose which creatures attack this turn. You choose which creatures block this turn and how those creatures block."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_collision_of_realms_uses_consult_and_bottom_remainder() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Collision of Realms")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each player shuffles all creatures they own into their library. Each player who shuffled a nontoken creature into their library this way reveals cards from the top of their library until they reveal a creature card, then puts that card onto the battlefield and the rest on the bottom of their library in a random order.",
        )
        .expect("Collision of Realms should parse");

    let raw = format!("{:?}", def.spell_effect);
    assert!(
        raw.contains("ForPlayersEffect")
            && raw.contains("TagMatchingObjectsEffect")
            && raw.contains("MoveToZoneEffect")
            && raw.contains("zone: Library")
            && raw.contains("ShuffleLibraryEffect")
            && raw.contains("ConsultTopOfLibraryEffect")
            && raw.contains("PutTaggedRemainderOnLibraryBottomEffect"),
        "expected Collision of Realms to keep its tagged shuffle-and-consult structure, got {raw}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("each player shuffles all creatures they own into their library")
            && rendered_lower
                .contains("who shuffled a nontoken creature into their library this way")
            && rendered_lower.contains("until they reveal a creature card")
            && rendered_lower.contains("that card onto the battlefield")
            && rendered_lower.contains("the rest on the bottom of their library in a random order"),
        "expected Collision of Realms oracle-like text to stay close to the oracle, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_last_march_of_the_ents_draws_greatest_toughness() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Last March Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw cards equal to the greatest toughness among creatures you control.")
        .expect("greatest-toughness draw clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("Draw") && debug.contains("GreatestToughness"),
        "expected draw count based on greatest toughness, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_flourishing_hunter_gains_life_equal_to_greatest_toughness() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Flourishing Hunter")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, you gain life equal to the greatest toughness among other creatures you control.",
        )
        .expect("Flourishing Hunter-style greatest-toughness life gain should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains(
            "gain life equal to the greatest toughness among other creatures you control"
        ),
        "expected greatest-toughness life-gain text to survive rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_search_target_opponents_library_face_down_exile_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Praetor's Grasp Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search target opponent's library for a card and exile it face down. Then that player shuffles.",
        )
        .expect("search target-opponent library face-down exile should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("chooser: You")
            && debug.contains("owner: Some(Target(")
            && debug.contains("Opponent")
            && debug.contains("face_down: true"),
        "expected controller-chooses opponent-library face-down exile search, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cast_this_spell_as_though_it_had_flash_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Necromancy Variant")
        .card_types(vec![CardType::Enchantment])
        .parse_text("You may cast this spell as though it had flash.")
        .expect("cast-this-spell-as-though-it-had-flash line should parse");

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::Flash),
        "expected flash static ability, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_borne_upon_a_wind_flash_permission_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Borne Upon a Wind")
        .card_types(vec![CardType::Instant])
        .parse_text("You may cast spells this turn as though they had flash.\nDraw a card.")
        .expect("borne upon a wind clause should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("GrantBySpecEffect")
            && debug.contains("You")
            && debug.contains("Flash")
            && debug.contains("DrawCardsEffect"),
        "expected temporary flash grant plus draw, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_tidal_barracuda_any_player_flash_permission_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Tidal Barracuda")
        .card_types(vec![CardType::Creature])
        .parse_text("Any player may cast spells as though they had flash.")
        .expect("any-player flash permission should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("any player may cast spells as though they had flash")
            || rendered.contains("players may cast spells as though they had flash"),
        "expected static flash permission text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn ian_malcolm_renders_source_exiled_cast_permission_with_any_mana() {
    let def = parse_oracle_card_definition("Ian Malcolm, Chaotician");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "During each player's turn, that player may cast a spell from among the cards they don't own exiled with this creature, and mana of any type can be spent to cast it."
        ),
        "expected source-exiled cast permission and any-mana rider to merge, got {rendered}"
    );

    let static_ids: Vec<_> = def
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability) => Some(static_ability.id()),
            _ => None,
        })
        .collect();
    assert!(
        static_ids.contains(&StaticAbilityId::Grants)
            && static_ids.contains(&StaticAbilityId::ManaSpendPermission),
        "expected grant plus mana-spend static abilities, got {static_ids:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn last_voyage_renders_sticker_aura_count_and_attached_sacrifice() {
    let def = parse_oracle_card_definition("Last Voyage of the _____");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    let lower = rendered.to_ascii_lowercase();
    assert!(
        lower.contains(
            "when this enchantment enters, you may put a name sticker on it, then it becomes an aura with enchant creature. return a creature card from your graveyard to the battlefield and attach this aura to it."
        ),
        "expected sticker/aura/return/attach sequence to compact, got {rendered}"
    );
    assert!(
        lower.contains(
            "enchanted creature gets +2/+0 for each name sticker on this aura with seven or fewer letters."
        ),
        "expected sticker-count anthem wording, got {rendered}"
    );
    assert!(
        lower.contains("when this aura leaves the battlefield, sacrifice enchanted creature."),
        "expected aura leaves trigger to sacrifice enchanted creature, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("StickersOnSource")
            && debug.contains("NameSticker")
            && debug.contains("max_name_letters: Some(7)"),
        "expected sticker-count anthem model, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn hunted_by_the_family_renders_villainous_choice_for_each_target() {
    let def = parse_oracle_card_definition("Hunted by The Family");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains(
            "Choose up to four target creatures you don't control. For each of them, that creature's controller faces a villainous choice — That creature becomes a 1/1 white Human creature and loses all abilities, or you create a token that's a copy of it."
        ),
        "expected for-each-target villainous choice wording, got {rendered}"
    );

    let debug = format!("{:#?}", def.spell_effect);
    assert!(
        debug.contains("ForEachTaggedEffect")
            && debug.contains("VillainousChoiceEffect")
            && debug.contains("CreateTokenCopyEffect")
            && debug.contains("RemoveAllAbilities"),
        "expected reusable villainous-choice model, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_valley_floodcaller_keeps_flash_grant_and_them_reference_wording() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Valley Floodcaller")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flash\nYou may cast noncreature spells as though they had flash.\nWhenever you cast a noncreature spell, Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them.",
        )
        .expect("valley floodcaller text should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("You may cast noncreature spells as though they had flash."),
        "expected noncreature flash grant wording, got {rendered}"
    );
    assert!(
        rendered.contains("Untap them."),
        "expected tagged follow-up untap wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valley_floodcaller_compiled_lines_meet_strict_semantic_threshold() {
    let oracle = "Flash\nYou may cast noncreature spells as though they had flash.\nWhenever you cast a noncreature spell, Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Valley Floodcaller")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("valley floodcaller text should parse");

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
        "expected Valley Floodcaller to clear strict semantic threshold, got score={similarity}, lines={compiled:?}"
    );
    assert!(
        !mismatch,
        "expected Valley Floodcaller to avoid semantic mismatch, got lines={compiled:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valley_rotcaller_strict_oracle_parses_and_renders_type_count() {
    assert_oracle_card_parses_strict("Valley Rotcaller");

    let oracle = oracle_text_by_name()
        .get("Valley Rotcaller")
        .expect("missing Valley Rotcaller oracle text")
        .clone();
    let def = parse_oracle_card_definition("Valley Rotcaller");
    let compiled = unprocessed_compiled_lines(&def);
    let rendered = compiled.join(" ").to_ascii_lowercase();

    assert!(
        rendered.contains("each opponent loses x life and you gain x life"),
        "expected Valley Rotcaller to render its life-drain attack trigger, got {rendered}"
    );
    assert!(
        rendered.contains("where x is the number of other")
            && rendered.contains("squirrels")
            && rendered.contains("bats")
            && rendered.contains("lizards")
            && rendered.contains("rats")
            && rendered.contains("you control"),
        "expected Valley Rotcaller to preserve the comma-separated creature-type count, got {rendered}"
    );

    let (_oracle_cov, _compiled_cov, similarity, _delta, mismatch) =
        crate::semantic_compare::compare_semantics_scored(
            &oracle,
            &compiled,
            Some(crate::semantic_compare::EmbeddingConfig {
                dims: 384,
                mismatch_threshold: 0.99,
            }),
        );
    assert!(
        similarity >= 0.99,
        "expected Valley Rotcaller to clear strict semantic threshold, got score={similarity}, lines={compiled:?}"
    );
    assert!(
        !mismatch,
        "expected Valley Rotcaller to avoid semantic mismatch, got lines={compiled:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn valley_rotcaller_creature(
    game: &mut crate::game_state::GameState,
    controller: PlayerId,
    name: &str,
    subtype: Subtype,
) -> ObjectId {
    let def = CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .subtypes(vec![subtype])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    game.create_object_from_definition(&def, controller, Zone::Battlefield)
}

#[cfg(ironsmith_runtime_parser_tests)]
pub(super) fn resolve_valley_rotcaller_attack(
    game: &mut crate::game_state::GameState,
    rotcaller: ObjectId,
) {
    let bob = PlayerId::from_index(1);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::combat::CreatureAttackedEvent::new(
            rotcaller,
            crate::triggers::AttackEventTarget::Player(bob),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut trigger_queue = crate::triggers::TriggerQueue::new();
    for entry in crate::triggers::check_triggers(game, &event)
        .into_iter()
        .filter(|entry| entry.source == rotcaller)
    {
        trigger_queue.add(entry);
    }
    assert_eq!(
        trigger_queue.entries.len(),
        1,
        "Valley Rotcaller should trigger when it attacks"
    );
    crate::game_loop::put_triggers_on_stack(game, &mut trigger_queue)
        .expect("Valley Rotcaller trigger should go on the stack");
    let mut dm = crate::decision::AutoPassDecisionMaker;
    crate::game_loop::resolve_stack_entry_with(game, &mut dm)
        .expect("Valley Rotcaller trigger should resolve");
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valley_rotcaller_attack_trigger_counts_other_supported_types_only() {
    let def = parse_oracle_card_definition("Valley Rotcaller");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rotcaller = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.object_mut(rotcaller)
        .expect("Valley Rotcaller should exist")
        .subtypes
        .push(Subtype::Squirrel);

    valley_rotcaller_creature(&mut game, alice, "Squirrel Ally", Subtype::Squirrel);
    valley_rotcaller_creature(&mut game, alice, "Bat Ally", Subtype::Bat);
    valley_rotcaller_creature(&mut game, alice, "Lizard Ally", Subtype::Lizard);
    valley_rotcaller_creature(&mut game, alice, "Rat Ally", Subtype::Rat);
    valley_rotcaller_creature(&mut game, alice, "Wizard Ally", Subtype::Wizard);
    valley_rotcaller_creature(&mut game, bob, "Opponent Rat", Subtype::Rat);

    resolve_valley_rotcaller_attack(&mut game, rotcaller);

    assert_eq!(
        game.life_total(bob),
        16,
        "Bob should lose 4 life for Alice's other Squirrel, Bat, Lizard, and Rat"
    );
    assert_eq!(
        game.life_total(alice),
        24,
        "Alice should gain 4 life from Valley Rotcaller's trigger"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn valley_rotcaller_attack_trigger_ignores_itself_when_no_other_type_matches() {
    let def = parse_oracle_card_definition("Valley Rotcaller");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let rotcaller = game.create_object_from_definition(&def, alice, Zone::Battlefield);
    game.object_mut(rotcaller)
        .expect("Valley Rotcaller should exist")
        .subtypes
        .push(Subtype::Squirrel);

    resolve_valley_rotcaller_attack(&mut game, rotcaller);

    assert_eq!(
        game.life_total(bob),
        20,
        "Valley Rotcaller should not count itself as another matching creature"
    );
    assert_eq!(
        game.life_total(alice),
        20,
        "Alice should not gain life when X is 0"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_zealous_display_non_turn_conditional_untap_clause() {
    let oracle = "Creatures you control get +2/+0 until end of turn. If it's not your turn, untap those creatures.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Zealous Display")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("Zealous Display conditional untap clause should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        (rendered_lower.contains("creatures you control get +2/+0 until end of turn")
            || rendered_lower.contains("each creature you control gets +2/+0 until end of turn"))
            && rendered_lower.contains("if it's not your turn")
            && rendered_lower.contains("untap those creatures"),
        "expected Zealous Display wording to preserve conditional untap follow-up, got {rendered}"
    );

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
        "expected Zealous Display to clear strict semantic threshold, got score={similarity}, lines={compiled:?}"
    );
    assert!(
        !mismatch,
        "expected Zealous Display to avoid semantic mismatch, got lines={compiled:?}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_color_then_add_devotion_to_that_color() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Nykthos Variant")
        .card_types(vec![CardType::Land])
        .parse_text("{2}, {T}: Choose a color. Add an amount of mana of that color equal to your devotion to that color.")
        .expect("choose-color devotion mana ability should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("ChooseColorEffect")
            && debug.contains("AddManaOfChosenColorEffect")
            && debug.contains("DevotionToChosenColor"),
        "expected choose-color plus devotion-to-chosen-color mana sequence, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_oriss_grandeur_named_discard_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Oriss, Samite Guardian")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Grandeur — Discard another card named Oriss, Samite Guardian: Target player can't cast spells this turn, and creatures that player controls can't attack this turn.",
        )
        .expect("grandeur named discard cost should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("DiscardEffect") && debug.contains("oriss, samite guardian"),
        "expected named-card discard cost, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("grandeur")
            && rendered.contains("discard another card named oriss, samite guardian"),
        "expected named-card grandeur cost in compiled text, got {rendered}"
    );
    assert!(
        rendered.contains(
            "target player can't cast spells this turn, and creatures that player controls can't attack this turn"
        ),
        "expected both restrictions to share the targeted player, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_page_loose_leaf_grandeur_keeps_named_discard_cost() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Page, Loose Leaf")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{T}: Add {C}.\nGrandeur — Discard another card named Page, Loose Leaf: Reveal cards from the top of your library until you reveal an instant or sorcery card. Put that card into your hand and the rest on the bottom of your library in a random order.",
        )
        .expect("Page, Loose Leaf should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("grandeur")
            && rendered.contains("discard another card named page, loose leaf"),
        "expected named-card grandeur cost in compiled text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_skoa_embermage_grandeur_keeps_named_discard_and_sacrifice_costs() {
    let oracle = "When Skoa enters, it deals 4 damage to any target.\nGrandeur — Discard another card named Skoa, Embermage, Sacrifice two Mountains: Skoa deals 4 damage to any target.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Skoa, Embermage")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Goblin, Subtype::Wizard])
        .power_toughness(PowerToughness::fixed(4, 4))
        .parse_text(
            "When Skoa enters, it deals 4 damage to any target.\nDiscard another card named Skoa, Embermage, Sacrifice two Mountains: Skoa deals 4 damage to any target.",
        )
        .expect("Skoa, Embermage should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("grandeur")
            && rendered.contains("discard another card named skoa, embermage"),
        "expected named-card grandeur cost, got {rendered}"
    );
    assert!(
        rendered.contains("sacrifice two mountains"),
        "expected mountain-sacrifice grandeur cost, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities).to_ascii_lowercase();
    assert!(
        debug.contains("skoa, embermage") && debug.contains("sacrificeeffect"),
        "expected named discard plus sacrifice cost structure, got {debug}"
    );

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
        similarity >= 0.88,
        "expected Skoa, Embermage wording similarity to improve after preserving named grandeur costs, got score={similarity}, lines={compiled:?}"
    );
    let _ = mismatch;
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_jackal_familiar_attack_or_block_alone_uses_alone_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Jackal Familiar")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack or block alone.")
        .expect("jackal familiar restriction should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("AttackOrBlockAlone"),
        "expected attack-or-block-alone restriction, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("can't attack or block alone")
            || rendered.contains("cant attack or block alone"),
        "expected alone restriction text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_bonded_horncrest_attack_or_block_alone_uses_alone_restriction() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Bonded Horncrest")
        .card_types(vec![CardType::Creature])
        .parse_text("This creature can't attack or block alone.")
        .expect("bonded horncrest restriction should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("AttackOrBlockAlone"),
        "expected attack-or-block-alone restriction, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_flamescroll_celebrant_non_mana_ability_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Flamescroll Celebrant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever an opponent activates an ability that isn't a mana ability, this creature deals 1 damage to that player.",
        )
        .expect("non-mana ability activation trigger should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("AbilityActivatedTrigger") && debug.contains("non_mana_only: true"),
        "expected qualified ability-activated trigger, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_coercion_choose_card_from_it_uses_tagged_hand_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Coercion")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Target opponent reveals their hand. You choose a card from it. That player discards that card.")
        .expect("coercion hand-choice chain should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:?}");
    assert!(
        debug.contains("LookAtHandEffect"),
        "expected reveal-hand setup, got {debug}"
    );
    assert!(
        debug.contains("ChooseObjectsEffect")
            && debug.contains("zone: Some(Hand)")
            && debug.contains("chooser: You"),
        "expected a non-targeted hand choice for you, got {debug}"
    );
    assert!(
        debug.contains("IsTaggedObject"),
        "expected chosen card filter to stay linked to the revealed hand, got {debug}"
    );
    assert!(
        debug.contains("Discard"),
        "expected follow-up discard effect, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_from_revealed_hand_then_graveyard_exiles_both_choices() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Dreams Variant")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Target opponent reveals their hand. You choose an artifact or creature card from it, then choose an artifact or creature card from their graveyard. Exile the chosen cards.",
        )
        .expect("dual hand/graveyard choice chain should parse");

    let effects = def.spell_effect.as_ref().expect("spell effects");
    let debug = format!("{effects:#?}");
    assert!(
        debug.contains("LookAtHandEffect"),
        "expected reveal-hand setup, got {debug}"
    );
    assert_eq!(
        debug.matches("ChooseObjectsEffect").count(),
        2,
        "expected one hand choice and one graveyard choice, got {debug}"
    );
    assert!(
        debug.contains("Hand") && debug.contains("Graveyard"),
        "expected choices from hand and graveyard, got {debug}"
    );
    assert!(
        debug.contains("Artifact") && debug.contains("Creature"),
        "expected artifact-or-creature filters, got {debug}"
    );
    assert!(
        debug.contains("MoveToZoneEffect") && debug.contains("zone: Exile"),
        "expected chosen cards to be exiled, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("reveals their hand")
            && rendered.contains("choose an artifact or creature card from it")
            && rendered.contains("choose an artifact or creature card from their graveyard")
            && rendered.contains("exile the chosen cards"),
        "expected dual-choice wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_kutzil_power_greater_than_base_power_trigger() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Kutzil Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever one or more creatures you control each with power greater than its base power deals combat damage to a player, draw a card.",
        )
        .expect("base-power comparison trigger subject should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("power greater than its base power"),
        "expected base-power comparison to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_an_opponent_then_that_player_cant_cast_spells() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Xanathar Restriction Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("When this creature enters, choose an opponent. That player can't cast spells this turn.")
        .expect("choose-opponent then cant-cast should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ChoosePlayerEffect")
            && abilities_debug.contains("filter: Opponent"),
        "expected choose-opponent effect, got {abilities_debug}"
    );
    assert!(
        abilities_debug.contains("CastSpellsMatching(TaggedPlayer")
            || abilities_debug.contains("CastSpellsMatching(IteratedPlayer"),
        "expected that-player cant-cast restriction to lower through existing player filters, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_enchant_player_upkeep_trigger_uses_attached_player_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Curse Variant")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .parse_text(
            "Enchant player\nAt the beginning of enchanted player's upkeep, that player loses 1 life.",
        )
        .expect("enchant-player curse text should parse");

    assert_eq!(
        def.aura_attach_filter,
        Some(AuraAttachmentFilter::Player(PlayerFilter::Any))
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchant player")
            && rendered.contains("at the beginning of enchanted player's upkeep")
            && rendered.contains("that player loses 1 life"),
        "expected enchant-player curse text to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_grievous_wound_oracle_tracks_enchanted_player_damage_trigger() {
    let def = parse_oracle_card_definition("Grievous Wound");

    assert_eq!(
        def.aura_attach_filter,
        Some(AuraAttachmentFilter::Player(PlayerFilter::Any))
    );

    let abilities_debug = format!("{:#?}", def.abilities);
    assert!(
        abilities_debug.contains("DealsDamageTrigger")
            && abilities_debug.contains("TaggedPlayer")
            && abilities_debug.contains("enchanted")
            && abilities_debug.contains("DamagedPlayer")
            && abilities_debug.contains("HalfLifeTotalRoundedUp"),
        "expected Grievous Wound to bind enchanted-player damage to the damaged player, got {abilities_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join("\n")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("enchant player")
            && rendered.contains("enchanted player can't gain life")
            && rendered.contains(
                "whenever enchanted player is dealt damage, that player loses half their life, rounded up"
            ),
        "expected Grievous Wound compiled text to preserve the enchant-player damage trigger, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_megatron_life_lost_turn_mana_clause() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Megatron Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "More Than Meets the Eye {1}{R}{W}{B} (You may cast this card converted for {1}{R}{W}{B}.)\nYour opponents can't cast spells during combat.\nAt the beginning of each of your postcombat main phases, you may convert Megatron. If you do, add {C} for each 1 life your opponents have lost this turn.",
        )
        .expect("life-lost mana clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("more than meets the eye {1}{r}{w}{b}")
            && rendered.contains("during combat")
            && rendered.contains("convert")
            && !rendered.contains("transform")
            && rendered.contains("add {c} for each 1 life your opponents have lost this turn")
            && !rendered.contains("time(s)"),
        "expected megatron silence and mana text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_convert_with_followup_sentence_preserves_convert_action() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Jetfire Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("Convert this creature, then adapt 3.")
        .expect("convert with followup should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("convert this creature") && rendered.contains("adapt 3"),
        "expected convert and followup text to survive compilation, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_craft_keyword_line_as_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Craft Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text_allow_unsupported(
            "Craft with artifact {3}{W}{W} ({3}{W}{W}, Exile this artifact, Exile another artifact you control or an artifact card from your graveyard: Return this card transformed under its owner's control. Craft only as a sorcery.)",
        )
        .expect("craft should parse as a supported activated keyword ability");

    let debug = format!("{def:#?}");
    assert!(
        debug.contains("Activated")
            && debug.contains("EmitKeywordActionEffect")
            && debug.contains("Craft")
            && debug.contains("TransformEffect"),
        "craft should lower to an activated ability, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_craft_with_creature_keyword_line_as_activated_ability() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Craft Creature Variant")
        .card_types(vec![CardType::Artifact])
        .parse_text_allow_unsupported(
            "Craft with creature {5}{G}{G} ({5}{G}{G}, Exile this artifact, Exile a creature you control or a creature card from your graveyard: Return this card transformed under its owner's control. Craft only as a sorcery.)",
        )
        .expect("craft with creature should parse as a supported activated keyword ability");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Craft with creature {5}{G}{G}"),
        "craft with creature should render structurally, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sphinxs_decree_next_turn_silence() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Sphinx's Decree")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Each opponent can't cast instant or sorcery spells during that player's next turn.",
        )
        .expect("sphinx's decree silence clause should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("each opponent")
            && rendered.contains("next upkeep")
            && rendered.contains("instant or sorcery spells"),
        "expected next-turn silence to lower through next upkeep, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn compiled_static_restriction_keeps_during_turn_condition_text() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Grand Abolisher")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "During your turn, your opponents can't cast spells or activate abilities of artifacts, creatures, or enchantments.",
        )
        .expect("grand abolisher should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("during your turn")
            && rendered.contains("your opponents can't cast spells"),
        "expected compiled text to keep during-your-turn condition, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_traveling_chocobo_top_library_lines_compile() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Traveling Chocobo")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "You may look at the top card of your library any time.\nYou may play lands and cast Bird spells from the top of your library.\nIf a land or Bird you control entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
        )
        .expect("traveling chocobo text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered
            .to_ascii_lowercase()
            .contains("top card of your library")
            && rendered.contains("Bird spells from the top of your library")
            && rendered
                .to_ascii_lowercase()
                .contains("triggers an additional time"),
        "expected top-of-library and trigger-doubling text, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cemetery_illuminator_top_library_source_exiled_type_permission() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Cemetery Illuminator")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Blue],
            vec![ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Spirit])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "Flying\nWhenever this creature enters or attacks, exile a card from a graveyard.\nYou may look at the top card of your library any time.\nOnce each turn, you may cast a spell from the top of your library if it shares a card type with a card exiled with this creature.",
        )
        .expect("Cemetery Illuminator should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("once each turn")
            && rendered_lower.contains("cast")
            && rendered_lower.contains("from the top of your library")
            && rendered_lower.contains("shares a card type with a card exiled with this creature"),
        "expected source-exiled top-library cast permission in compiled text, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("LookAtTopCardOfLibrary")
            && debug.contains("PlayFrom")
            && debug.contains("OnceEachTurn")
            && debug.contains(crate::tag::SOURCE_EXILED_TAG)
            && debug.contains("SharesCardType"),
        "expected Cemetery Illuminator to lower into a limited top-library PlayFrom grant keyed to source-exiled card types, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_guardian_naga_banishing_coils_creature_face_strict() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Guardian Naga // Banishing Coils")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Vigilance\nDuring your turn, prevent all damage that would be dealt to this creature.",
        )
        .expect("guardian naga creature face should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("vigilance")
            && rendered.contains("during your turn")
            && rendered.contains("prevent all damage that would be dealt to this creature"),
        "expected guardian naga prevention clause to compile, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_guardian_naga_banishing_coils_adventure_face_strict() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Guardian Naga // Banishing Coils")
        .card_types(vec![CardType::Instant])
        .parse_text("Exile target artifact or enchantment.")
        .expect("banishing coils adventure face should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("exile target artifact or enchantment"),
        "expected banishing coils exile clause to compile, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_starfield_vocalist_with_warp_keyword() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Starfield Vocalist")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If a permanent entering the battlefield causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.\nWarp {1}{U} (You may cast this card from your hand for its warp cost. Exile this creature at the beginning of the next end step, then you may cast it from exile on a later turn.)",
        )
        .expect("starfield vocalist text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Warp {1}{U}")
            && rendered
                .to_ascii_lowercase()
                .contains("triggers an additional time"),
        "expected warp keyword plus trigger doubling, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wulfgar_of_icewind_dale_with_melee_keyword() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Wulfgar of Icewind Dale")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Melee\nIf a creature you control attacking causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
        )
        .expect("wulfgar text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered.contains("Melee") && rendered_lower.contains("triggers an additional time"),
        "expected melee keyword plus trigger doubling, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_gandalf_flash_union_uses_generic_permission_parser() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Gandalf Variant")
        .card_types(vec![CardType::Creature])
        .parse_text("You may cast legendary spells and artifact spells as though they had flash.")
        .expect("gandalf-style flash union should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.to_ascii_lowercase().contains("legendary")
            && rendered.to_ascii_lowercase().contains("artifact")
            && rendered.to_ascii_lowercase().contains("flash")
            && !rendered.to_ascii_lowercase().contains("noncreature"),
        "expected generic flash-union rendering, got {rendered}"
    );

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("any_of")
            && debug.contains("Legendary")
            && debug.contains("Artifact")
            && !debug.contains("RuleFallbackText"),
        "expected shared permission filter union without placeholders, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn strength_of_will_compiled_text_keeps_target_and_granted_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Strength of Will")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Until end of turn, target creature you control gains indestructible and \"Whenever this creature is dealt damage, put that many +1/+1 counters on it.\"",
        )
        .expect("strength of will text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("target creature you control gains indestructible")
            && rendered.contains("whenever this creature is dealt damage")
            && rendered.contains("put that many +1/+1 counters on it"),
        "expected targeted granted trigger rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn enter_the_avatar_state_keeps_shared_duration_and_targeted_subtype_gain() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Enter the Avatar State")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Until end of turn, target creature you control becomes an Avatar in addition to its other types and gains flying, first strike, lifelink, and hexproof.",
        )
        .expect("Enter the Avatar State should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("AddSubtypes")
            && spell_debug.contains("Avatar")
            && spell_debug.contains("AddAbility")
            && spell_debug.matches("EndOfTurn").count() >= 2,
        "expected targeted subtype and keyword grants to share until-end-of-turn duration, got {spell_debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        !rendered.contains("unsupported effect")
            && rendered.contains(
                "target creature you control becomes an avatar in addition to its other types"
            )
            && rendered.contains("gains flying")
            && rendered.contains("first strike")
            && rendered.contains("lifelink")
            && rendered.contains("hexproof")
            && rendered.matches("until end of turn").count() >= 1,
        "expected clean compiled text for Enter the Avatar State, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn scuttling_sentinel_parses_blue_crab_until_end_of_turn_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(651_777), "Scuttling Sentinel")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(1)],
            vec![ManaSymbol::Green, ManaSymbol::Blue],
            vec![ManaSymbol::Green, ManaSymbol::Blue],
        ]))
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Crab, Subtype::Elf])
        .power_toughness(PowerToughness::fixed(3, 2))
        .parse_text(
            "Flash\nVigilance\nWhen this creature enters, put a +1/+1 counter on another target creature you control. Until end of turn, that creature becomes a blue Crab in addition to its other types and gains hexproof.",
        )
        .expect("Scuttling Sentinel should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("when this creature enters")
            && rendered_lower.contains("another target creature you control")
            && rendered_lower.contains(
                "that creature becomes a blue crab in addition to its other types and gains hexproof until end of turn"
            )
            && !rendered_lower.contains("unsupported"),
        "expected Scuttling Sentinel to render its blue Crab hexproof trigger cleanly, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("SetColors")
            && debug.contains("AddSubtypes")
            && debug.contains("Crab")
            && debug.contains("Hexproof")
            && debug.matches("EndOfTurn").count() >= 3,
        "expected Scuttling Sentinel trigger to structurally set color, add Crab, and grant hexproof until end of turn, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn anti_venom_static_damage_replacement_compiles() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Anti-Venom, Horrifying Healer")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "If damage would be dealt to Anti-Venom, prevent that damage and put that many +1/+1 counters on him.",
        )
        .expect("anti-venom replacement text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent that damage")
            && rendered.contains("put that many +1/+1 counters"),
        "expected damage replacement rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn amy_rose_attach_to_her_uses_source_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Amy Rose")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Red],
            vec![ManaSymbol::White],
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .parse_text(
            "Haste\nWhenever Amy Rose attacks, attach up to one target Equipment to her. Then up to one other target attacking creature gets +X/+0 until end of turn, where X is Amy Rose's power.",
        )
        .expect("amy rose should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("AttachObjectsEffect") && debug.contains("target: Source"),
        "expected attachment target to resolve to the source, got {debug}"
    );
    assert!(
        debug.contains("FullName(\"Amy Rose\")"),
        "expected Amy Rose's power to preserve a named-source surface hint, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn test_of_faith_renders_prevention_follow_up_counters() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Test of Faith")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Prevent the next 3 damage that would be dealt to target creature this turn. For each 1 damage prevented this way, put a +1/+1 counter on that creature.",
        )
        .expect("test of faith text should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("prevent the next 3 damage")
            && rendered.contains("for each 1 damage prevented this way")
            && rendered.contains("put a +1/+1 counter on that creature"),
        "expected prevention follow-up rendering, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn jared_carthalion_true_heir_compiles_monarch_and_damage_replacement_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Jared Carthalion, True Heir")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When Jared Carthalion enters, target opponent becomes the monarch. You can't become the monarch this turn.\nIf damage would be dealt to Jared Carthalion while you're the monarch, prevent that damage and put that many +1/+1 counters on it.",
        )
        .expect("jared rules text should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    let lowered = rendered.to_ascii_lowercase();
    assert!(
        lowered.contains("target opponent becomes the monarch")
            && lowered.contains("you can't become the monarch this turn")
            && (lowered
                .contains("if damage would be dealt to jared carthalion while you're the monarch")
                || lowered.contains(
                    "if damage would be dealt to this creature while you're the monarch"
                ))
            && lowered.contains("prevent that damage")
            && lowered.contains("put that many +1/+1 counters on it")
            && !lowered.contains("unsupported effect"),
        "expected Jared to render monarch and prevention text cleanly, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sokenzan_renegade_keeps_unique_hand_leader_upkeep_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sokenzan Renegade")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Bushido 1 (Whenever this creature blocks or becomes blocked, it gets +1/+1 until end of turn.)\nAt the beginning of your upkeep, if a player has more cards in hand than each other player, the player who has the most cards in hand gains control of this creature.",
        )
        .expect("Sokenzan Renegade should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Bushido 1")
            && rendered.contains(
                "At the beginning of your upkeep, if a player has more cards in hand than each other player, the player who has the most cards in hand gains control of this creature."
            ),
        "expected oracle-like upkeep trigger rendering, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerHasMoreCardsInHandThanEachOtherPlayer { player: Any }")
            && debug.contains("ChangeControllerToPlayer(MostCardsInHand)"),
        "expected unique hand-leader trigger lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_wild_dogs_keeps_unique_life_leader_upkeep_trigger() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Wild Dogs")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 1))
        .parse_text(
            "At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature.\nCycling {2} ({2}, Discard this card: Draw a card.)",
        )
        .expect("Wild Dogs should parse");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains(
            "At the beginning of your upkeep, if a player has more life than each other player, the player with the most life gains control of this creature."
        ) && rendered.contains("Cycling {2}"),
        "expected oracle-like upkeep trigger rendering, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PlayerHasMoreLifeThanEachOtherPlayer { player: Any }")
            && debug.contains("ChangeControllerToPlayer(MostLifeTied)"),
        "expected unique life-leader trigger lowering, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn chaos_lord_parses_even_permanent_control_trigger_and_haste_as_though_text() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(2_614), "Chaos Lord")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Human])
        .power_toughness(PowerToughness::fixed(7, 7))
        .parse_text(
            "First strike\nAt the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even.\nThis creature can attack as though it had haste unless it entered this turn.",
        )
        .expect("Chaos Lord should parse strictly");

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("First strike")
            && rendered.contains(
                "At the beginning of your upkeep, target opponent gains control of this creature if the number of permanents is even."
            )
            && rendered.contains(
                "This creature can attack as though it had haste unless it entered this turn."
            ),
        "expected Chaos Lord to render its even-permanent control trigger and as-though attack clause, got {rendered}"
    );

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CountParity")
            && debug.contains("ChangeControllerToPlayer")
            && debug.contains("CanAttackAsThoughHaste")
            && debug.contains("ObjectEnteredBattlefieldThisTurn"),
        "expected Chaos Lord to lower parity and haste-unless-entered structurally, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_lulu_loyal_hollyphant_keeps_revolt_gate_and_untap_followup() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Lulu, Loyal Hollyphant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Flying\nAt the beginning of your end step, if a permanent you controlled left the battlefield this turn, put a +1/+1 counter on each tapped creature you control, then untap them.\nChoose a Background (You can have a Background as a second commander.)",
        )
        .expect("Lulu, Loyal Hollyphant should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("PermanentLeftBattlefieldUnderYourControlThisTurn")
            && debug.contains("UntapEffect"),
        "expected Lulu to keep both the revolt-style gate and untap followup, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("At the beginning of your end step")
            && (rendered.contains("a permanent left the battlefield under your control this turn")
                || rendered.contains("a permanent you controlled left the battlefield this turn"))
            && rendered.contains("put a +1/+1 counter on each tapped creature you control")
            && (rendered.contains("Untap them") || rendered.contains("Untap those creatures")),
        "expected Lulu oracle-like trigger rendering to keep the gate and untap followup, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_cloakwood_hermit_keeps_creature_card_graveyard_gate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Cloakwood Hermit")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Background])
        .parse_text(
            "Commander creatures you own have \"At the beginning of your end step, if a creature card was put into your graveyard from anywhere this turn, create two tapped 1/1 green Squirrel creature tokens.\"",
        )
        .expect("Cloakwood Hermit should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("CreatureCardPutIntoYourGraveyardThisTurn"),
        "expected Cloakwood Hermit to keep the creature-card graveyard gate, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("at the beginning of your end step")
            && rendered
                .contains("a creature card was put into your graveyard from anywhere this turn")
            && rendered.contains("create two tapped 1/1 green squirrel creature tokens"),
        "expected Cloakwood Hermit trigger and gate wording to render, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_choose_background_renders_keyword_line() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Background Partner Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Choose a Background (You can have a Background as a second commander.)")
        .expect("choose-a-background line should parse");

    let rendered = unprocessed_compiled_lines(&def).join("\n");
    assert!(
        rendered.contains("Choose a Background"),
        "expected Background partner line to render as keyword text, got {rendered}"
    );
    assert!(
        !rendered.contains("You choose a Background you control"),
        "Background partner line should not render as a battlefield choice effect, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_power_based_draw_renders_cards_equal_to_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Shadowheart Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "{1}{B}, {T}, Sacrifice another creature: Draw X cards, where X is that creature's power.",
        )
        .expect("power-based draw activation should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw cards equal to that creature's power")
            || rendered.contains("draw x cards, where x is that creature's power"),
        "expected power-based draw count to render as cards equal to power, got {rendered}"
    );
    assert!(
        !rendered.contains("power cards"),
        "power-based draw should not render the power phrase as a card count, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn souls_majesty_parses_with_target_power_draw_count() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soul's Majesty")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw cards equal to the power of target creature you control.")
        .expect("Soul's Majesty should parse");

    let spell = def.spell_effect.as_ref().expect("should have spell effect");
    let effects = &spell.segments[0].default_effects;
    let debug = format!("{:?}", effects);
    assert!(
        debug.contains("DrawCardsEffect")
            && debug.contains("PowerOf")
            && debug.contains("controller: Some(You)")
            && debug.contains("card_types: [Creature]"),
        "expected Soul's Majesty to compile to target-creature-you-control power draw, got {debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn souls_majesty_compiled_text_mentions_target_creature_you_control_power() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soul's Majesty")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw cards equal to the power of target creature you control.")
        .expect("Soul's Majesty should parse");

    let rendered = crate::compiled_text::canonical_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw cards equal to")
            && rendered.contains("target creature you control")
            && rendered.contains("power"),
        "expected Soul's Majesty compiled text to preserve target/power clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn souls_majesty_draws_equal_to_target_power_including_zero_power_branch() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soul's Majesty")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(4)],
            vec![ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Sorcery])
        .parse_text("Draw cards equal to the power of target creature you control.")
        .expect("Soul's Majesty should parse");
    let spell_effect = def.spell_effect.as_ref().expect("should have spell effect");

    let mut game =
        crate::game_state::GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let soul_id = game.create_object_from_definition(&def, alice, Zone::Battlefield);

    let big_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(2), "Big Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let small_creature = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(3), "Small Target")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 2))
            .build(),
        alice,
        Zone::Battlefield,
    );

    for i in 0..8 {
        game.create_object_from_card(
            &crate::card::CardBuilder::new(CardId::from_raw(10 + i), &format!("Library {i}"))
                .card_types(vec![CardType::Creature])
                .build(),
            alice,
            Zone::Library,
        );
    }

    let hand_before = game.player(alice).expect("alice should exist").hand.len();
    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(soul_id, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(big_creature)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        soul_id,
        spell_effect,
        None,
        &[],
    )
    .expect("Soul's Majesty should resolve targeting a 4-power creature");

    let hand_after_big = game.player(alice).expect("alice should exist").hand.len();
    assert_eq!(
        hand_after_big,
        hand_before + 4,
        "Soul's Majesty should draw cards equal to the target creature's power"
    );

    let mut dm = crate::decision::AutoPassDecisionMaker;
    let mut ctx = crate::effects::ExecutionContext::new(soul_id, alice, &mut dm)
        .with_targets(vec![crate::effects::ResolvedTarget::Object(small_creature)]);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        soul_id,
        spell_effect,
        None,
        &[],
    )
    .expect("Soul's Majesty should resolve targeting a 0-power creature");

    let hand_after_zero = game.player(alice).expect("alice should exist").hand.len();
    assert_eq!(
        hand_after_zero, hand_after_big,
        "Soul's Majesty should draw zero cards when the target creature's power is zero"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_spellcast_mana_value_boost_renders_that_spell_reference() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Livaan Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "Whenever you cast a noncreature spell, target creature gets +X/+0 until end of turn, where X is that spell's mana value.",
        )
        .expect("spellcast mana-value boost should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("where x is that spell's mana value"),
        "expected spellcast trigger to render that-spell mana value, got {rendered}"
    );
    assert!(
        !rendered.contains("card in that player's hand's mana value"),
        "spellcast trigger should not render as a hand-card mana value, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_graveyard_threshold_preserves_there_are_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Viconia Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, if there are four or more creature cards in your graveyard, return a creature card at random from your graveyard to your hand.",
        )
        .expect("graveyard-threshold trigger should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("if there are four or more creature cards in your graveyard"),
        "expected graveyard threshold to keep there-are wording, got {rendered}"
    );
    assert!(
        !rendered.contains("if you have four or more creature cards in your graveyard"),
        "there-are graveyard threshold should not collapse to you-have wording, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_draw_power_then_gain_toughness_keeps_both_effects() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Momentous Fall Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nYou draw cards equal to the sacrificed creature's power, then you gain life equal to its toughness.",
        )
        .expect("power/toughness draw-gain chain should parse");

    let debug = format!("{:?}", def.spell_effect);
    assert!(
        debug.contains("DrawCardsEffect") && debug.contains("GainLifeEffect"),
        "expected draw and gain-life effects to survive comma-then chain parsing, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw cards equal to the sacrificed creature's power")
            && rendered.contains("then you gain life equal to its toughness"),
        "expected power/toughness draw-gain pair to render as one linked sentence, got {rendered}"
    );
    assert!(
        rendered.contains("as an additional cost to cast this spell, sacrifice a creature"),
        "expected additional-cost sacrifice wording to omit redundant control clause, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_additional_sacrifice_draw_power_uses_sacrificed_creature_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Life's Legacy Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "As an additional cost to cast this spell, sacrifice a creature.\nDraw cards equal to the sacrificed creature's power.",
        )
        .expect("additional-cost sacrificed-power draw should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("draw cards equal to the sacrificed creature's power"),
        "expected additional-cost sacrificed creature reference, got {rendered}"
    );
    assert!(
        !rendered.contains("that creature's power"),
        "additional-cost sacrificed creature reference should not render as a generic that-creature reference, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_target_creature_opponent_controls_keeps_oracle_surface() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Severed Strands Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Destroy target creature an opponent controls.")
        .expect("opponent-controlled target creature should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered.contains("destroy target creature an opponent controls"),
        "expected opponent-controlled target creature surface, got {rendered}"
    );
    assert!(
        !rendered.contains("target opponent's creature"),
        "opponent-controlled target creature should not render as a possessive target opponent's creature, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_power_based_counters_render_x_where_clause() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Soul's Might Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Put X +1/+1 counters on target creature, where X is that creature's power.")
        .expect("power-based counter count should parse");

    let rendered = unprocessed_compiled_lines(&def)
        .join(" ")
        .to_ascii_lowercase();
    assert!(
        rendered
            .contains("put x +1/+1 counters on target creature, where x is that creature's power"),
        "expected dynamic counter count to render with an X where-clause, got {rendered}"
    );
    assert!(
        !rendered.contains("power +1/+1 counter"),
        "dynamic counter count should not render the power phrase as a counter count, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_sarevok_deathbringer_keeps_global_ltb_gate_and_player_loss() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Sarevok, Deathbringer")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .parse_text(
            "At the beginning of each player's end step, if no permanents left the battlefield this turn, that player loses X life, where X is Sarevok's power.\nChoose a Background (You can have a Background as a second commander.)",
        )
        .expect("Sarevok, Deathbringer should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeginningOfEndStepTrigger { player: Any, surface: Definite }")
            && debug.contains("intervening_if: Some(Not(PermanentLeftBattlefieldThisTurn))")
            && debug.contains("LoseLifeEffect")
            && debug.contains("PowerOf")
            && debug.contains("Source"),
        "expected Sarevok to keep the global leave-the-battlefield gate and life-loss effect, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("At the beginning of the end step")
            && rendered.contains("if no permanents left the battlefield this turn")
            && (rendered.contains("that player loses X life")
                || rendered.contains("that player loses life equal to this creature's power"))
            && (rendered.contains("this creature's power")
                || rendered.contains("Sarevok's power")
                || rendered.contains("its power")),
        "expected Sarevok oracle-like rendering to preserve the gate and loss text, got {rendered}"
    );

    let compiled = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
    assert!(
        (compiled.contains("At the beginning of each end step")
            || compiled.contains("At the beginning of each player's end step"))
            && compiled.contains("if no permanents left the battlefield this turn")
            && (compiled.contains("that player loses X life")
                || compiled.contains("that player loses life equal to this creature's power"))
            && (compiled.contains("this creature's power")
                || compiled.contains("Sarevok's power")
                || compiled.contains("its power"))
            && !compiled.contains("if not"),
        "expected Sarevok compiled text to render the negated condition clearly, got {compiled}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn definite_end_step_surface_is_preserved_for_frozen_card_cluster() {
    for (name, text) in [
        (
            "Arc Runner",
            "At the beginning of the end step, sacrifice this creature.",
        ),
        (
            "Ichorid",
            "At the beginning of the end step, sacrifice this creature.",
        ),
        (
            "Impetuous Devils",
            "At the beginning of the end step, sacrifice this creature.",
        ),
        (
            "Kuon, Ogre Ascendant",
            "At the beginning of the end step, if three or more creatures died this turn, flip this creature.",
        ),
    ] {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), name)
            .card_types(vec![CardType::Creature])
            .parse_text(text)
            .unwrap_or_else(|error| panic!("{name} should parse: {error:?}"));
        let debug = format!("{:?}", def.abilities);
        assert!(
            debug.contains("BeginningOfEndStepTrigger { player: Any, surface: Definite }"),
            "{name} should keep the definite typed end-step surface: {debug}"
        );
        let rendered = unprocessed_compiled_lines(&def).join(" ");
        assert!(
            rendered.starts_with("At the beginning of the end step,"),
            "{name} should render the definite end-step surface: {rendered}"
        );
    }

    let each = CardDefinitionBuilder::new(CardId::from_raw(2), "Each End Step Control")
        .card_types(vec![CardType::Creature])
        .parse_text("At the beginning of each end step, sacrifice this creature.")
        .expect("each-end-step control should parse");
    let rendered = unprocessed_compiled_lines(&each).join(" ");
    assert!(
        rendered.starts_with("At the beginning of each end step,"),
        "explicit each-end-step wording must remain distinct: {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn cant_be_blocked_frozen_clusters_render_as_single_oracle_clauses() {
    for (name, expected) in [
        (
            "Distortion Strike",
            "target creature gets +1/+0 until end of turn and can't be blocked this turn",
        ),
        (
            "Hraesvelgr of the First Brood",
            "target creature gets +1/+0 until end of turn and can't be blocked this turn",
        ),
        (
            "Taigam's Strike",
            "target creature gets +2/+0 until end of turn and can't be blocked this turn",
        ),
        (
            "Teleportal",
            "target creature you control gets +1/+0 until end of turn and can't be blocked this turn",
        ),
        (
            "Temmet, Vizier of Naktamun",
            "target creature token you control gets +1/+1 until end of turn and can't be blocked this turn",
        ),
    ] {
        let compiled = canonical_compiled_lines(&parse_oracle_card_definition(name))
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            compiled.contains(expected),
            "{name} should coalesce its same-target pump and unblockable effects: {compiled}"
        );
    }

    for name in ["Expendable Lackey", "Reservoir Kraken"] {
        let compiled = canonical_compiled_lines(&parse_oracle_card_definition(name))
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            compiled.contains(
                "create a 1/1 blue fish creature token with \"this token can't be blocked\""
            ),
            "{name} should inline the permanent token ability: {compiled}"
        );
        assert!(
            !compiled.contains("it has \"this token can't be blocked\""),
            "{name} should not split the token ability into a follow-up sentence: {compiled}"
        );
    }

    for (name, expected) in [
        (
            "Ichor Synthesizer",
            "as long as this has four or more oil counters on it, this gets +2/+0 and can't be blocked",
        ),
        (
            "Jace's Sentinel",
            "as long as you control a jace planeswalker, this gets +1/+0 and can't be blocked",
        ),
        (
            "Slippery Scoundrel",
            "as long as you have the city's blessing, this has hexproof and can't be blocked",
        ),
        (
            "Steel of the Godhead",
            "as long as enchanted creature is blue, enchanted creature gets +1/+1 and can't be blocked",
        ),
        (
            "Vortex Runner",
            "as long as you control eight or more lands, this gets +1/+0 and can't be blocked",
        ),
    ] {
        let compiled = canonical_compiled_lines(&parse_oracle_card_definition(name))
            .join("\n")
            .to_ascii_lowercase();
        assert!(
            compiled.contains(expected),
            "{name} should coalesce same-condition continuous bonuses: {compiled}"
        );
    }
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn parse_kitsune_mystic_keeps_two_aura_intervening_if_gate() {
    let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Kitsune Mystic")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 3))
        .parse_text(
            "At the beginning of the end step, if this creature is enchanted by two or more Auras, flip it.",
        )
        .expect("Kitsune Mystic should parse");

    let debug = format!("{:?}", def.abilities);
    assert!(
        debug.contains("BeginningOfEndStepTrigger { player: Any }")
            && debug.contains("intervening_if: Some(CountComparison")
            && debug.contains("AttachedToSource")
            && debug.contains("Aura")
            && debug.contains("GreaterThanOrEqual(2)")
            && debug.contains("FlipEffect"),
        "expected Kitsune Mystic to keep the two-Aura gate and flip effect, got {debug}"
    );

    let rendered = unprocessed_compiled_lines(&def).join(" ");
    assert!(
        (rendered.contains("At the beginning of each end step")
            || rendered.contains("At the beginning of each player's end step"))
            && rendered
                .to_ascii_lowercase()
                .contains("if this creature is enchanted by two or more auras")
            && rendered.contains("flip it"),
        "expected Kitsune Mystic rendering to preserve the two-Aura gate, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
pub(super) fn descend_end_step_cards_keep_intervening_if_gate_and_surface() {
    for name in [
        "Broodrage Mycoid",
        "Canonized in Blood",
        "Child of the Volcano",
        "Deep Goblin Skulltaker",
        "Enterprising Scallywag",
        "Ruin-Lurker Bat",
    ] {
        let def = parse_oracle_card_definition(name);
        let debug = format!("{:?}", def.abilities);
        assert!(
            debug.contains("BeginningOfEndStepTrigger { player: You }")
                && debug.contains("intervening_if: Some(PlayerDescendedThisTurn { player: You })"),
            "expected {name} to keep the typed descend intervening-if gate, got {debug}"
        );

        let rendered = canonical_compiled_lines(&def).join(" ");
        assert!(
            rendered.contains("At the beginning of your end step, if you descended this turn"),
            "expected {name} to render the descend intervening-if clause, got {rendered}"
        );
    }
}
