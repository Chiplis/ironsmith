//! Card database module for MTG.
//!
//! This module provides a structured way to define cards with their abilities.
//! Cards are defined programmatically for type safety and LLM-friendliness.
//!
//! Each card is defined in its own file under `definitions/` for easy tracking.

pub mod builders;
#[cfg(any(test, feature = "handwritten-parse-support"))]
pub mod definitions;
mod helper_tags;
pub mod tokens;

pub(crate) use builders::CardDefinitionBuilder;
#[cfg(any(test, feature = "handwritten-parse-support"))]
pub use definitions::*;
pub use helper_tags::is_sentence_helper_tag;

#[cfg(test)]
mod parse_snapshots;

mod generated_registry {
    include!(concat!(env!("OUT_DIR"), "/generated_registry.rs"));
}

mod generated_meld_counterparts {
    include!(concat!(env!("OUT_DIR"), "/generated_meld_counterparts.rs"));
}

use crate::ability::Ability;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cost::OptionalCost;
use crate::effect::Effect;
#[path = "../../../ironsmith-registry/src/runtime_registry_impl.rs"]
mod registry_impl;

pub use registry_impl::*;
pub type CardDefinition = ironsmith_core::CardDefinition<
    Ability,
    Effect,
    crate::costs::Cost,
    AlternativeCastingMethod,
    OptionalCost,
>;

pub trait CardDefinitionRuntimeExt {
    fn additional_non_mana_costs(&self) -> Vec<crate::costs::Cost>;
}

impl CardDefinitionRuntimeExt for CardDefinition {
    fn additional_non_mana_costs(&self) -> Vec<crate::costs::Cost> {
        fn presentation_cost(cost: &crate::costs::Cost) -> crate::costs::Cost {
            let Some(effect) = cost.effect_ref() else {
                return cost.clone();
            };
            if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
                return crate::costs::Cost::try_from_runtime_effect(*tagged.effect.clone())
                    .unwrap_or_else(|_| cost.clone());
            }
            if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
                return crate::costs::Cost::try_from_runtime_effect(*with_id.effect.clone())
                    .unwrap_or_else(|_| cost.clone());
            }
            cost.clone()
        }

        self.additional_cost
            .non_mana_costs()
            .map(presentation_cost)
            .collect()
    }
}

#[cfg(test)]
pub(crate) fn register_builtin_handwritten_cards_if_for_runtime_tests<F>(
    registry: &mut CardRegistry,
    mut include_constructor_key: F,
) where
    F: FnMut(&str) -> bool,
{
    macro_rules! maybe_register {
        ($constructor:ident) => {
            if include_constructor_key(stringify!($constructor)) {
                registry.register($constructor());
            }
        };
    }

    maybe_register!(llanowar_elves);
    maybe_register!(chrome_mox);
    maybe_register!(command_the_mind);
    maybe_register!(serra_angel);
    maybe_register!(grizzly_bears);
    maybe_register!(lightning_bolt);
    maybe_register!(doom_blade);
    maybe_register!(demonic_tutor);
    maybe_register!(enlightened_tutor);
    maybe_register!(emrakul_the_promised_end);
    maybe_register!(everflowing_chalice);
    maybe_register!(force_of_will);
    maybe_register!(giant_growth);
    maybe_register!(mindbreak_trap);
    maybe_register!(counterspell);
    maybe_register!(dawn_charm);
    maybe_register!(demonic_consultation);
    maybe_register!(swords_to_plowshares);
    maybe_register!(basic_forest);
    maybe_register!(basic_island);
    maybe_register!(basic_mountain);
    maybe_register!(basic_plains);
    maybe_register!(basic_swamp);
    maybe_register!(ornithopter);
    maybe_register!(murder_of_crows);
    maybe_register!(goblin_guide);
    maybe_register!(typhoid_rats);
    maybe_register!(vampire_nighthawk);
    maybe_register!(silhana_ledgewalker);
    maybe_register!(thorn_elemental);
    maybe_register!(mirran_crusader);
    maybe_register!(crusade);
    maybe_register!(stormbreath_dragon);
    maybe_register!(geist_of_saint_traft);
    maybe_register!(savannah_lions);
    maybe_register!(savines_reclamation);
    maybe_register!(saw_in_half);
    maybe_register!(white_knight);
    maybe_register!(giant_spider);
    maybe_register!(wall_of_omens);
    maybe_register!(boggart_brute);
    maybe_register!(darksteel_colossus);
    maybe_register!(snapcaster_mage);
    maybe_register!(underworld_breach);
    maybe_register!(frogmite);
    maybe_register!(treasure_cruise);
    maybe_register!(trinisphere);
    maybe_register!(stoke_the_flames);
    maybe_register!(reverse_engineer);
    maybe_register!(the_birth_of_meletis);
    maybe_register!(thassas_oracle);
    maybe_register!(student_of_warfare);
    maybe_register!(valley_floodcaller);
    maybe_register!(yawgmoth_thran_physician);
    maybe_register!(yawgmoths_will);
    maybe_register!(butcher_ghoul);
    maybe_register!(sightless_ghoul);
    maybe_register!(hex_parasite);
    maybe_register!(fireball);
    maybe_register!(think_twice);
    maybe_register!(urzas_saga);
    maybe_register!(fate_transfer);
    maybe_register!(accursed_marauder);
    maybe_register!(accursed_duneyard);
    maybe_register!(akromas_will);
    maybe_register!(amulet_of_vigor);
    maybe_register!(ancient_tomb);
    maybe_register!(arcane_signet);
    maybe_register!(arid_mesa);
    maybe_register!(ashaya_soul_of_the_wild);
    maybe_register!(ashnods_altar);
    maybe_register!(bello_bard_of_the_brambles);
    maybe_register!(black_lotus);
    maybe_register!(blade_of_the_bloodchief);
    maybe_register!(bleachbone_verge);
    maybe_register!(blood_celebrant);
    maybe_register!(blood_artist);
    maybe_register!(bloodstained_mire);
    maybe_register!(bosh_iron_golem);
    maybe_register!(braids_arisen_nightmare);
    maybe_register!(breaking);
    maybe_register!(entering);
    maybe_register!(brightclimb_pathway);
    maybe_register!(grimclimb_pathway);
    maybe_register!(buried_alive);
    maybe_register!(cataclysm);
    maybe_register!(cataclysmic_gearhulk);
    maybe_register!(charismatic_conqueror);
    maybe_register!(conquerors_galleon);
    maybe_register!(conquerors_foothold);
    maybe_register!(command_tower);
    maybe_register!(sol_ring);
    maybe_register!(scrubland);
    maybe_register!(tainted_field);
    maybe_register!(high_market);
    maybe_register!(humility);
    maybe_register!(vampiric_tutor);
    maybe_register!(flooded_strand);
    maybe_register!(mana_tithe);
    maybe_register!(marsh_flats);
    maybe_register!(polluted_delta);
    maybe_register!(rebuff_the_wicked);
    maybe_register!(verdant_catacombs);
    maybe_register!(windswept_heath);
    maybe_register!(yasharn_implacable_earth);
    maybe_register!(lightning_greaves);
    maybe_register!(selfless_spirit);
    maybe_register!(serum_powder);
    maybe_register!(mother_of_runes);
    maybe_register!(giver_of_runes);
    maybe_register!(selfless_savior);
    maybe_register!(gods_willing);
    maybe_register!(kami_of_false_hope);
    maybe_register!(krrik_son_of_yawgmoth);
    maybe_register!(shelter);
    maybe_register!(mox_diamond);
    maybe_register!(mox_sapphire);
    maybe_register!(library_of_leng);
    maybe_register!(invisible_stalker);
    maybe_register!(dauthi_slayer);
    maybe_register!(zodiac_rooster);
    maybe_register!(culling_the_weak);
    maybe_register!(fleshbag_marauder);
    maybe_register!(generous_gift);
    maybe_register!(gemstone_caverns);
    maybe_register!(godless_shrine);
    maybe_register!(hanweir_battlements);
    maybe_register!(hanweir_garrison);
    maybe_register!(hanweir_the_writhing_township);
    maybe_register!(innocent_blood);
    maybe_register!(mana_vault);
    maybe_register!(maskwood_nexus);
    maybe_register!(merciless_executioner);
    maybe_register!(phyrexian_tower);
    maybe_register!(shattered_sanctum);
    maybe_register!(stroke_of_midnight);
    maybe_register!(tainted_pact);
    maybe_register!(vault_of_champions);
    maybe_register!(tayam_luminous_enigma);
    maybe_register!(village_rites);
    maybe_register!(model_of_unity);
    maybe_register!(manascape_refractor);
    maybe_register!(squirrel_nest);
    maybe_register!(mycosynth_lattice);
    maybe_register!(nest_of_scarabs);
    maybe_register!(toph_the_first_metalbender);
    maybe_register!(marneus_calgar);
    maybe_register!(marvin_murderous_mimic);
    maybe_register!(rex_cyber_hound);
    maybe_register!(tivit_seller_of_secrets);
    maybe_register!(wall_of_roots);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::AbilityKind;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;
    #[cfg(feature = "generated-registry")]
    use crate::{game_state::GameState, ids::PlayerId};

    #[test]
    fn handwritten_builtin_card_constructors_compile_through_parser() {
        let module_source = include_str!("mod.rs");
        let constructor_names = registered_handwritten_constructor_names(module_source);
        assert!(
            !constructor_names.is_empty(),
            "expected handwritten registry to register at least one constructor"
        );

        let definitions_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards/definitions");
        let definition_sources = std::fs::read_dir(&definitions_dir)
            .expect("read cards/definitions directory")
            .filter_map(|entry| {
                let entry = entry.expect("read definitions directory entry");
                let path = entry.path();
                (path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                    && path.file_name().and_then(|name| name.to_str()) != Some("mod.rs")
                    && path.file_name().and_then(|name| name.to_str()) != Some("builder.rs"))
                .then(|| {
                    (
                        path.display().to_string(),
                        std::fs::read_to_string(&path).expect("read card definition source"),
                    )
                })
            })
            .collect::<Vec<_>>();

        let mut failures = Vec::new();
        for constructor_name in constructor_names {
            match handwritten_constructor_body(&definition_sources, &constructor_name) {
                Some((path, body)) if body.contains(".parse_text(") => {
                    let _ = path;
                }
                Some((path, _body)) => failures.push(format!(
                    "{constructor_name} in {path} does not call .parse_text("
                )),
                None => failures.push(format!(
                    "{constructor_name} is registered but no pub fn {constructor_name}() -> CardDefinition body was found"
                )),
            }
        }

        assert!(
            failures.is_empty(),
            "handwritten builtin card constructors must compile rules text through the parser:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn handwritten_builtin_cards_have_no_card_specific_compiler_hooks() {
        let module_source = include_str!("mod.rs");
        let constructor_names = registered_handwritten_constructor_names(module_source);
        let definitions_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cards/definitions");
        let definition_sources = std::fs::read_dir(&definitions_dir)
            .expect("read cards/definitions directory")
            .filter_map(|entry| {
                let entry = entry.expect("read definitions directory entry");
                let path = entry.path();
                (path.extension().and_then(|ext| ext.to_str()) == Some("rs")
                    && path.file_name().and_then(|name| name.to_str()) != Some("mod.rs")
                    && path.file_name().and_then(|name| name.to_str()) != Some("builder.rs"))
                .then(|| {
                    (
                        path.display().to_string(),
                        std::fs::read_to_string(&path).expect("read card definition source"),
                    )
                })
            })
            .collect::<Vec<_>>();

        let mut card_terms = Vec::new();
        for constructor_name in constructor_names {
            let Some((_path, body)) =
                handwritten_constructor_body(&definition_sources, &constructor_name)
            else {
                continue;
            };
            if let Some(name) = handwritten_constructor_card_name(body) {
                let name = name.to_ascii_lowercase();
                if is_distinctive_card_hook_term(&name) {
                    card_terms.push(name);
                }
            }
        }
        card_terms.sort();
        card_terms.dedup();

        let compiler_src = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root")
            .join("crates/ironsmith-compiler/src/runtime_backend");
        let mut stack = vec![compiler_src];
        let mut hits = Vec::new();
        while let Some(path) = stack.pop() {
            for entry in std::fs::read_dir(&path).expect("read compiler source") {
                let path = entry.expect("read compiler entry").path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if !path.extension().is_some_and(|ext| ext == "rs") {
                    continue;
                }
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "tests.rs" || name.ends_with("_tests.rs"))
                {
                    continue;
                }

                let source = strip_cfg_test_source(
                    &std::fs::read_to_string(&path).expect("read compiler file"),
                )
                .to_ascii_lowercase();
                for (line_index, line) in source.lines().enumerate() {
                    for term in &card_terms {
                        if line.contains(term) {
                            hits.push(format!(
                                "{}:{} contains {:?}",
                                path.display(),
                                line_index + 1,
                                term
                            ));
                        }
                    }
                }
            }
        }

        assert!(
            hits.is_empty(),
            "handwritten builtin cards must not compile through card-specific compiler hooks:\n{}",
            hits.join("\n")
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn handwritten_builtin_effect_sentences_route_through_subject_verb() {
        let constructor_names = registered_handwritten_constructor_names(include_str!("mod.rs"));
        let mut failures = Vec::new();

        for constructor_name in constructor_names {
            let (_registry, trace) = ironsmith_compiler::parse_trace::capture(|| {
                let mut registry = CardRegistry::new();
                register_builtin_handwritten_cards_if_for_runtime_tests(
                    &mut registry,
                    |candidate| candidate == constructor_name,
                );
                registry
            });

            let rendered = trace.render();
            let effect_sentence_count = rendered
                .lines()
                .filter(|line| line.contains("effect sentence:"))
                .filter(|line| !line.contains("effect sentence: \"\""))
                .count();
            if effect_sentence_count == 0 {
                continue;
            }

            let subject_verb_count = rendered.matches("effect-route: subject-verb").count();
            let non_subject_routes = rendered
                .lines()
                .filter_map(|line| line.trim().strip_prefix("effect-route: "))
                .filter(|route| !route.starts_with("subject-verb"))
                .map(str::to_string)
                .collect::<Vec<_>>();

            if subject_verb_count < effect_sentence_count || !non_subject_routes.is_empty() {
                failures.push(format!(
                    "{constructor_name}: effect_sentences={effect_sentence_count}, subject_verb={subject_verb_count}, non_subject_routes={non_subject_routes:?}"
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "all handwritten builtin effect sentences must route through Subject/Verb:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn migrated_semantic_islands_route_through_generic_subject_verb_programs() {
        let cases = [
            (
                "Generic Extra Turn",
                "Take an extra turn after this one.",
                "subject-verb verb=Take subject=implicit recognizer=extra-turn-after-anchor",
            ),
            (
                "Generic Damage Prevention",
                "Prevent all damage that would be dealt this turn to target creature.",
                "subject-verb verb=Prevent subject=implicit recognizer=damage-prevention",
            ),
            (
                "Generic Monstrosity",
                "Monstrosity 3.",
                "subject-verb verb=Monstrosity subject=implicit recognizer=keyword-action",
            ),
            (
                "Generic Earthbend",
                "Earthbend 2.",
                "subject-verb verb=Earthbend subject=implicit recognizer=keyword-action",
            ),
            (
                "Generic Enchant",
                "Enchant creature.",
                "subject-verb verb=Enchant subject=implicit recognizer=aura-attachment",
            ),
            (
                "Generic Permission",
                "Until end of turn, you may play lands and cast spells from your graveyard.",
                "subject-verb verb=Play subject=explicit recognizer=zone-permission",
            ),
            (
                "Generic Replacement",
                "If a card would be put into your graveyard from anywhere this turn, exile that card instead.",
                "subject-verb verb=Exile subject=implicit recognizer=instead-replacement",
            ),
            (
                "Generic Choice Complement",
                "Each player chooses from among the permanents they control an artifact, a creature, an enchantment, and a land, then sacrifices the rest.",
                "subject-verb verb=Choose subject=explicit recognizer=choice-complement-sacrifice",
            ),
            (
                "Generic Flashback Grant",
                "Target card in your graveyard gains flashback until end of turn. The flashback cost is equal to its mana cost.",
                "subject-verb verb=Gain subject=explicit recognizer=parameterized-flashback-grant",
            ),
            (
                "Generic Library Iteration",
                "Exile the top card of your library. You may put that card into your hand unless it has the same name as another card exiled this way. Repeat this process until you put a card into your hand or you exile two cards with the same name, whichever comes first.",
                "subject-verb verb=Exile subject=explicit recognizer=iterative-library-procedure",
            ),
            (
                "Generic Vote Procedure",
                "Starting with you, each player votes for death or taxes. For each death vote, each opponent sacrifices a creature. For each taxes vote, each opponent discards a card.",
                "subject-verb verb=Vote subject=explicit recognizer=vote-procedure",
            ),
            (
                "Generic Meld",
                "Exile them, then meld them into Chittering Host.",
                "subject-verb verb=Meld subject=explicit recognizer=meld-result",
            ),
            (
                "Generic Combat Choice Control",
                "You choose which creatures attack this turn.",
                "subject-verb verb=Choose subject=explicit recognizer=combat-choice-control",
            ),
            (
                "Generic Damage Replacement Counters",
                "If damage would be dealt to target creature this turn, prevent that damage and put that many +1/+1 counters on it.",
                "subject-verb verb=Prevent subject=implicit recognizer=damage-replacement-counters",
            ),
            (
                "Generic Looked Cards Counted Remainder",
                "Look at the top three cards of your library, then put two of them into your hand and the rest into your graveyard.",
                "subject-verb verb=Look subject=explicit recognizer=counted-looked-cards-remainder",
            ),
            (
                "Generic Consult Reveal Until Hand",
                "Reveal cards from the top of your library until you reveal a nonland card, then put all cards revealed this way into your hand.",
                "subject-verb verb=Reveal subject=explicit recognizer=consult-reveal-until-hand",
            ),
            (
                "Generic Each Player Exile Top Cast",
                "Exile the top card of each player's library, then you may cast any number of spells from among those cards without paying their mana costs.",
                "subject-verb verb=Exile subject=explicit recognizer=each-player-exile-top-cast",
            ),
            (
                "Generic Cant Restriction",
                "Target creature can't block this turn.",
                "subject-verb verb=Cant subject=explicit recognizer=restriction",
            ),
            (
                "Generic Where X Binding",
                "Draw X cards, where X is the number of creatures you control.",
                "subject-verb verb=Bind subject=implicit recognizer=value-binding",
            ),
        ];

        let mut failures = Vec::new();
        for (name, text, expected_route) in cases {
            let (result, trace) = ironsmith_compiler::parse_trace::capture(|| {
                CardDefinitionBuilder::new(CardId::new(), name).parse_text(text)
            });
            if let Err(err) = result {
                failures.push(format!("{name}: parse failed: {err:?}"));
                continue;
            }
            let rendered = trace.render();
            if !rendered.contains(expected_route) {
                failures.push(format!(
                    "{name}: missing expected generic route {expected_route:?}\n{rendered}"
                ));
            }
        }

        assert!(
            failures.is_empty(),
            "migrated semantic islands must compile through generic Subject/Verb programs:\n{}",
            failures.join("\n")
        );
    }

    fn registered_handwritten_constructor_names(module_source: &str) -> Vec<String> {
        module_source
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                let name = line.strip_prefix("maybe_register!(")?.strip_suffix(");")?;
                Some(name.to_string())
            })
            .collect()
    }

    fn handwritten_constructor_body<'a>(
        sources: &'a [(String, String)],
        constructor_name: &str,
    ) -> Option<(&'a str, &'a str)> {
        let needle = format!("pub fn {constructor_name}(");
        for (path, source) in sources {
            let Some(start) = source.find(&needle) else {
                continue;
            };
            let signature_tail = &source[start..];
            let Some(open_offset) = signature_tail.find('{') else {
                continue;
            };
            let signature = &signature_tail[..open_offset];
            if !signature.contains("-> CardDefinition") {
                continue;
            }

            let body_start = start + open_offset;
            let Some(body_end) = matching_brace_end(source, body_start) else {
                continue;
            };
            return Some((&path[..], &source[body_start..=body_end]));
        }
        None
    }

    fn handwritten_constructor_card_name(body: &str) -> Option<String> {
        let builder_start = body.find("CardDefinitionBuilder::new(")?;
        let after_builder = &body[builder_start..];
        let first_quote = after_builder.find('"')?;
        let name_start = builder_start + first_quote + 1;
        let after_name_start = &body[name_start..];
        let name_end = name_start + after_name_start.find('"')?;
        Some(body[name_start..name_end].to_string())
    }

    fn is_distinctive_card_hook_term(name: &str) -> bool {
        name.contains(' ')
            && name.len() >= 8
            && !matches!(
                name,
                "conqueror's foothold" | "grimclimb pathway" | "basic forest" | "basic island"
            )
    }

    fn strip_cfg_test_source(source: &str) -> String {
        let mut stripped = String::new();
        let mut lines = source.lines().peekable();
        while let Some(line) = lines.next() {
            let trimmed = line.trim();
            if trimmed.starts_with("#[cfg(test)]")
                || trimmed.starts_with("#[cfg(all(test,")
                || trimmed.starts_with("#[cfg(any(test,")
            {
                while let Some(next) = lines.peek() {
                    if next.trim().is_empty() {
                        stripped.push('\n');
                        lines.next();
                    } else {
                        break;
                    }
                }
                if let Some(next) = lines.peek()
                    && next.trim_start().starts_with("mod tests")
                {
                    let test_line = lines.next().expect("peeked test module line");
                    stripped.push_str(&" ".repeat(test_line.len()));
                    stripped.push('\n');
                    if let Some(open_offset) = test_line.find('{') {
                        let mut depth = 1usize;
                        for byte in test_line.as_bytes()[open_offset + 1..].iter() {
                            match *byte {
                                b'{' => depth += 1,
                                b'}' => {
                                    depth = depth.saturating_sub(1);
                                }
                                _ => {}
                            }
                        }
                        while depth > 0 {
                            let Some(test_body_line) = lines.next() else {
                                break;
                            };
                            for byte in test_body_line.as_bytes() {
                                match *byte {
                                    b'{' => depth += 1,
                                    b'}' => {
                                        depth = depth.saturating_sub(1);
                                    }
                                    _ => {}
                                }
                            }
                            stripped.push_str(&" ".repeat(test_body_line.len()));
                            stripped.push('\n');
                        }
                    }
                    continue;
                }
            }
            stripped.push_str(line);
            stripped.push('\n');
        }
        stripped
    }

    fn matching_brace_end(source: &str, open_brace: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (offset, byte) in source.as_bytes()[open_brace..].iter().enumerate() {
            match *byte {
                b'{' => depth += 1,
                b'}' => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(open_brace + offset);
                    }
                }
                _ => {}
            }
        }
        None
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_card_definition_creation() {
        let def = llanowar_elves();
        assert_eq!(def.name(), "Llanowar Elves");
        assert!(def.is_creature());
        assert!(!def.abilities.is_empty());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_spell_definition() {
        let def = lightning_bolt();
        assert_eq!(def.name(), "Lightning Bolt");
        assert!(def.is_spell());
        assert!(def.spell_effect.is_some());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_registry_lookup() {
        let registry =
            CardRegistry::with_builtin_cards_for_names(["Serra Angel", "Lightning Bolt", "Forest"]);

        let angel = registry.get("Serra Angel");
        assert!(angel.is_some());
        assert!(angel.unwrap().is_creature());

        let bolt = registry.get("Lightning Bolt");
        assert!(bolt.is_some());
        assert!(bolt.unwrap().is_spell());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_registry_queries() {
        let registry =
            CardRegistry::with_builtin_cards_for_names(["Serra Angel", "Lightning Bolt", "Forest"]);

        let creatures: Vec<_> = registry.creatures().collect();
        assert!(!creatures.is_empty());

        let spells: Vec<_> = registry.spells().collect();
        assert!(!spells.is_empty());

        let lands: Vec<_> = registry.lands().collect();
        assert!(!lands.is_empty());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn test_registry_count() {
        let registry =
            CardRegistry::with_builtin_cards_for_names(["Serra Angel", "Lightning Bolt", "Forest"]);
        assert_eq!(registry.len(), 3);
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn ensure_cards_loaded_is_incremental() {
        let mut registry = CardRegistry::new();
        assert_eq!(registry.len(), 0);

        registry.ensure_cards_loaded(["Lightning Bolt"]);
        assert_eq!(registry.len(), 1);
        assert!(registry.get("Lightning Bolt").is_some());
        assert!(registry.get("Serra Angel").is_none());

        registry.ensure_cards_loaded(["Serra Angel"]);
        assert_eq!(registry.len(), 2);
        assert!(registry.get("Serra Angel").is_some());
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn ensure_cards_loaded_normalizes_input_names() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["  lightning bolt  ", " FoReSt "]);

        assert!(registry.get("Lightning Bolt").is_some());
        assert!(registry.get("Forest").is_some());
        assert_eq!(registry.len(), 2);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn try_compile_card_can_resolve_handwritten_cards_without_full_builtin_registry() {
        let definition = CardRegistry::try_compile_card("Lightning Bolt")
            .expect("handwritten card should compile");
        assert_eq!(definition.name(), "Lightning Bolt");
        assert!(definition.is_spell());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn try_compile_card_can_resolve_basic_lands_by_name() {
        let definition =
            CardRegistry::try_compile_card("Forest").expect("basic land should compile");
        assert_eq!(definition.name(), "Forest");
        assert!(definition.card.is_land());
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn ensure_cards_loaded_can_load_generated_cards() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["Conclave Evangelist"]);
        assert!(registry.get("Conclave Evangelist").is_some());
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn generated_registry_includes_transform_and_adventure_front_faces() {
        assert!(CardRegistry::generated_parser_card_parse_source("Jace, Vryn's Prodigy").is_some());
        assert!(CardRegistry::generated_parser_card_parse_source("Brazen Borrower").is_some());
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn try_compile_card_prefers_builtin_transform_pair_metadata() {
        let galleon = CardRegistry::try_compile_card("Conqueror's Galleon // Conqueror's Foothold")
            .expect("builtin transform pair should compile");
        let foothold = CardRegistry::try_compile_card("Conqueror's Foothold")
            .expect("builtin back face should compile");

        assert_eq!(
            galleon.card.other_face_name.as_deref(),
            Some("Conqueror's Foothold")
        );
        assert_eq!(
            foothold.card.other_face_name.as_deref(),
            Some("Conqueror's Galleon")
        );
        assert_eq!(
            galleon.card.other_face,
            Some(crate::ids::CardId::from_raw(234_002))
        );
        assert_eq!(
            foothold.card.other_face,
            Some(crate::ids::CardId::from_raw(234_001))
        );
        assert_eq!(
            galleon.card.linked_face_layout,
            crate::card::LinkedFaceLayout::TransformLike
        );
        assert_eq!(
            foothold.card.linked_face_layout,
            crate::card::LinkedFaceLayout::TransformLike
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn ensure_cards_loaded_can_load_adventure_front_face() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["Brazen Borrower"]);

        let borrower = registry
            .get("Brazen Borrower")
            .expect("adventure front face should load from generated registry");
        assert_eq!(borrower.card.name, "Brazen Borrower");
        assert!(borrower.card.is_creature());
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn ensure_cards_loaded_can_load_disturb_transform_front_face() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["Baithook Angler // Hook-Haunt Drifter"]);

        let angler = registry
            .get("Baithook Angler")
            .expect("disturb transform front face should load from generated registry");
        assert_eq!(angler.card.name, "Baithook Angler");
        assert_eq!(
            angler.card.other_face_name.as_deref(),
            Some("Hook-Haunt Drifter")
        );
        assert_eq!(
            angler.card.linked_face_layout,
            crate::card::LinkedFaceLayout::TransformLike
        );
        assert!(
            angler.alternative_casts.iter().any(|method| matches!(
                method,
                crate::alternative_cast::AlternativeCastingMethod::Disturb { .. }
            )),
            "disturb front face should expose a Disturb alternative"
        );
        assert!(
            registry.get("Hook-Haunt Drifter").is_some(),
            "disturb back face should be available after loading the family"
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn generated_registry_includes_split_cards_with_combined_aliases() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["Breaking // Entering"]);

        let front = registry
            .get("Breaking")
            .expect("split front face should load from generated registry");
        assert_eq!(
            front.card.linked_face_layout,
            crate::card::LinkedFaceLayout::Split
        );
        assert!(
            front.has_fuse,
            "fuse metadata should be preserved on split card"
        );

        assert!(
            registry.get("Breaking // Entering").is_some(),
            "combined split-card name should resolve via generated registry alias"
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn generated_registry_includes_flavor_name_aliases() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["You're Gonna Need a Bigger Boat", "Marauding Mutagen"]);

        assert_eq!(
            CardRegistry::generated_parser_card_parse_source("You're Gonna Need a Bigger Boat")
                .map(|(name, _)| name),
            Some("Abrade".to_string())
        );
        assert_eq!(
            CardRegistry::generated_parser_card_parse_source("Marauding Mutagen")
                .map(|(name, _)| name),
            Some("Acidic Slime".to_string())
        );

        assert!(registry.get("You're Gonna Need a Bigger Boat").is_some());
        assert!(registry.get("you're gonna need a bigger boat").is_some());
        assert!(registry.get("Marauding Mutagen").is_some());
        assert!(registry.get("marauding mutagen").is_some());

        let mut game = GameState::new(vec!["Alice".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let hand_definition = registry
            .get("You're Gonna Need a Bigger Boat")
            .expect("flavor alias should resolve")
            .clone();
        let hand_id = game.create_object_from_definition(&hand_definition, alice, Zone::Hand);
        assert_eq!(
            game.object(hand_id).expect("hand object should exist").name,
            "Abrade"
        );

        for alias in ["You're Gonna Need a Bigger Boat", "Marauding Mutagen"] {
            let definition = registry
                .get(alias)
                .expect("deck alias should resolve")
                .clone();
            game.create_object_from_definition(&definition, alice, Zone::Library);
        }

        let library_names: Vec<String> = game
            .player(alice)
            .expect("alice should exist")
            .library
            .iter()
            .filter_map(|&id| game.object(id).map(|object| object.name.clone()))
            .collect();
        assert!(
            library_names.iter().any(|name| name == "Abrade"),
            "expected canonical Abrade in library, got {library_names:?}"
        );
        assert!(
            library_names.iter().any(|name| name == "Acidic Slime"),
            "expected canonical Acidic Slime in library, got {library_names:?}"
        );
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn ensure_cards_loaded_skips_unsupported_generated_fallback_definitions() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["A Killer Among Us"]);
        assert!(
            registry.get("A Killer Among Us").is_none(),
            "unsupported generated fallback definitions should not be registered"
        );
    }

    #[test]
    fn meld_counterpart_name_uses_generated_pairs() {
        assert_eq!(
            meld_counterpart_name("Graf Rats"),
            Some("Midnight Scavengers")
        );
        assert_eq!(
            meld_counterpart_name("Midnight Scavengers"),
            Some("Graf Rats")
        );
        assert_eq!(meld_counterpart_name("Chittering Host"), None);
    }

    #[test]
    fn generated_definition_support_accepts_regular_definition() {
        let card = CardBuilder::new(CardId::new(), "Support Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let mut definition = CardDefinition::new(card);
        definition
            .abilities
            .push(Ability::static_ability(StaticAbility::flying()));

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_discard_this_card_activated_ability_as_hand_zone_ability() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Bloodrush Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{R}, Discard this card: Target attacking creature gets +3/+3 until end of turn",
            )
            .expect("discard-this-card activated ability should parse");

        let (ability, activated) = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some((ability, activated)),
                _ => None,
            })
            .expect("expected an activated ability");

        assert!(
            ability.functions_in(&Zone::Hand),
            "expected discard-this-card ability to function in hand"
        );
        assert!(
            !ability.functions_in(&Zone::Battlefield),
            "expected discard-this-card ability to not function on battlefield"
        );

        let costs = activated.mana_cost.display().to_ascii_lowercase();
        assert!(
            costs.contains("discard this card"),
            "expected activated cost to include discard-this-card, got: {costs}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_if_this_is_tapped_predicate_as_intervening_if() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Storage Land Probe")
            .card_types(vec![CardType::Land])
            .parse_text("At the beginning of your upkeep, if this land is tapped, put a storage counter on it.")
            .expect("tapped predicate trigger should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert_eq!(
            triggered.intervening_if,
            Some(crate::ConditionExpr::SourceIsTapped),
            "expected intervening-if to be SourceIsTapped"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_if_there_are_no_counters_on_this_predicate() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Depletion Land Probe")
            .card_types(vec![CardType::Land])
            .parse_text("If there are no depletion counters on this land, sacrifice it.")
            .expect("no-counters predicate should parse");

        // Ensure we actually produced an effect (not a dropped sentence).
        assert!(
            def.spell_effect
                .as_ref()
                .is_some_and(|effects| !effects.is_empty())
                || !def.abilities.is_empty(),
            "expected parsed effects or abilities"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_add_mana_for_each_counter_removed_this_way_uses_x_value() {
        use crate::ability::AbilityKind;
        use crate::effect::Value;
        use crate::effects::mana::AddScaledManaEffect;

        let def = CardDefinitionBuilder::new(CardId::new(), "Storage Land Probe")
            .card_types(vec![CardType::Land])
            .parse_text("{1}, Remove any number of storage counters from this land: Add {W} for each storage counter removed this way.")
            .expect("storage land mana scaling should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected an activated ability");

        let scaled = activated
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected scaled mana effect");

        assert_eq!(scaled.amount, Value::X);
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_activate_no_more_than_twice_each_turn_as_activation_limit() {
        use crate::ability::AbilityKind;

        let def = CardDefinitionBuilder::new(CardId::new(), "Activation Limit Probe")
            .card_types(vec![CardType::Creature])
            .parse_text("{B}: This creature gets +0/+1 until end of turn. Activate no more than twice each turn.")
            .expect("activation limit clause should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected an activated ability");

        assert_eq!(
            activated.activation_condition,
            Some(crate::ConditionExpr::MaxActivationsPerTurn(2))
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_flip_it_clause_as_flip_effect() {
        use crate::ability::AbilityKind;
        use crate::effects::{FlipEffect, MayEffect, SequenceEffect, TaggedEffect, WithIdEffect};

        fn contains_flip(effect: &crate::effect::Effect) -> bool {
            if effect.downcast_ref::<FlipEffect>().is_some() {
                return true;
            }
            if let Some(may) = effect.downcast_ref::<MayEffect>() {
                return may.effects.iter().any(contains_flip);
            }
            if let Some(seq) = effect.downcast_ref::<SequenceEffect>() {
                return seq.effects.iter().any(contains_flip);
            }
            if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
                return contains_flip(&tagged.effect);
            }
            if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
                return contains_flip(&with_id.effect);
            }
            false
        }

        let def = CardDefinitionBuilder::new(CardId::new(), "Flip Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "At the beginning of the end step, if there are two or more ki counters on this creature, you may flip it.",
            )
            .expect("flip clause should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert!(
            triggered.effects.iter().any(contains_flip),
            "expected FlipEffect in triggered effects"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_assigns_no_combat_damage_clause_as_combat_prevention() {
        use crate::ability::AbilityKind;
        use crate::effects::{
            IfEffect, MayEffect, PreventAllCombatDamageEffect, PreventAllCombatDamageFromEffect,
            SequenceEffect, TaggedEffect, WithIdEffect,
        };

        fn contains_prevent(effect: &crate::effect::Effect) -> bool {
            if effect
                .downcast_ref::<PreventAllCombatDamageEffect>()
                .is_some()
                || effect
                    .downcast_ref::<PreventAllCombatDamageFromEffect>()
                    .is_some()
            {
                return true;
            }
            if let Some(may) = effect.downcast_ref::<MayEffect>() {
                return may.effects.iter().any(contains_prevent);
            }
            if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
                return if_effect.then.iter().any(contains_prevent)
                    || if_effect.else_.iter().any(contains_prevent);
            }
            if let Some(seq) = effect.downcast_ref::<SequenceEffect>() {
                return seq.effects.iter().any(contains_prevent);
            }
            if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
                return contains_prevent(&tagged.effect);
            }
            if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
                return contains_prevent(&with_id.effect);
            }
            false
        }

        let def = CardDefinitionBuilder::new(CardId::new(), "Laccolith Probe")
            .card_types(vec![CardType::Creature])
            .parse_text("Whenever this creature becomes blocked, you may have it deal damage equal to its power to target creature. If you do, this creature assigns no combat damage this turn.")
            .expect("assigns-no-combat-damage clause should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert!(
            triggered.effects.iter().any(contains_prevent),
            "expected combat damage prevention in triggered effects, got {:#?}",
            triggered.effects
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_look_at_top_then_put_some_into_hand_rest_into_graveyard() {
        use crate::effects::{ChooseObjectsEffect, LookAtTopCardsEffect};

        let def = CardDefinitionBuilder::new(CardId::new(), "Ancestral Memories Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Look at the top seven cards of your library. Put two of them into your hand and the rest into your graveyard.",
            )
            .expect("look/put partition clause should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<LookAtTopCardsEffect>().is_some()),
            "expected LookAtTopCardsEffect in compiled effects"
        );
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ChooseObjectsEffect>().is_some()),
            "expected ChooseObjectsEffect in compiled effects"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_look_at_top_then_put_them_back_in_any_order() {
        use crate::effects::{LookAtTopCardsEffect, ReorderLibraryTopEffect};

        let def = CardDefinitionBuilder::new(CardId::new(), "Look Reorder Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Look at the top three cards of your library. Put them back in any order.")
            .expect("look/reorder clause should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<LookAtTopCardsEffect>().is_some()),
            "expected LookAtTopCardsEffect in compiled effects"
        );
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ReorderLibraryTopEffect>().is_some()),
            "expected ReorderLibraryTopEffect in compiled effects"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_discover_keyword_action_clause() {
        use crate::effects::DiscoverEffect;

        let def = CardDefinitionBuilder::new(CardId::new(), "Discover Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Discover 4.")
            .expect("discover clause should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<DiscoverEffect>().is_some()),
            "expected DiscoverEffect in compiled effects"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_become_basic_land_type_of_your_choice_until_eot() {
        use crate::effects::BecomeBasicLandTypeChoiceEffect;

        let def = CardDefinitionBuilder::new(CardId::new(), "Become Land Type Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{T}: Target land becomes the basic land type of your choice until end of turn.",
            )
            .expect("basic land type choice become clause should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(act) => Some(act),
                _ => None,
            })
            .expect("expected an activated ability");

        assert!(
            activated.effects.iter().any(|e| e
                .downcast_ref::<BecomeBasicLandTypeChoiceEffect>()
                .is_some()),
            "expected BecomeBasicLandTypeChoiceEffect in activated effects"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_can_block_additional_creature_each_combat_static_ability() {
        use crate::static_abilities::StaticAbilityId;

        let def = CardDefinitionBuilder::new(CardId::new(), "Extra Block Probe")
            .card_types(vec![CardType::Creature])
            .parse_text("This creature can block an additional creature each combat.")
            .expect("extra block static ability should parse");

        let has = def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(sa) => {
                sa.id() == StaticAbilityId::CanBlockAdditionalCreatureEachCombat
            }
            _ => false,
        });
        assert!(
            has,
            "expected CanBlockAdditionalCreatureEachCombat static ability"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn parse_enchanted_creature_cant_attack_or_block_static() {
        use crate::static_abilities::StaticAbilityId;

        let def = CardDefinitionBuilder::new(CardId::new(), "Aura Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text("Enchant creature\nEnchanted creature can't attack or block.")
            .expect("attached cant attack/block line should parse");

        assert!(
            def.aura_attach_filter.is_some(),
            "expected aura attach filter from 'Enchant creature' line"
        );

        let has = def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(sa) => sa.id() == StaticAbilityId::AttachedAbilityGrant,
            _ => false,
        });
        assert!(has, "expected AttachedAbilityGrant static ability on aura");
    }

    #[test]
    fn generated_definition_support_rejects_parser_fallback_markers() {
        let card = CardBuilder::new(CardId::new(), "Fallback Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::unsupported_parser_line(
            "probe text",
            "ParseError(\"mock\")",
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        assert!(!generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_reports_parser_fallback_reason() {
        let card = CardBuilder::new(CardId::new(), "Fallback Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::unsupported_parser_line(
            "probe text",
            "ParseError(\"unsupported ring clause (clause: 'Ring tempts')\")",
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        let message = generated_definition_unsupported_mechanics_message(&definition)
            .expect("expected unsupported message");
        assert!(
            message.contains("unsupported ring clause"),
            "expected unsupported reason in message, got {message}"
        );
    }

    #[test]
    fn generated_definition_support_flags_any_unsupported_marker_in_debug_output() {
        let card = CardBuilder::new(CardId::new(), "Unsupported Marker Probe")
            .oracle_text("Unsupported marker probe")
            .card_types(vec![CardType::Creature])
            .build();
        let definition = CardDefinition::new(card);

        assert!(
            generated_definition_has_unimplemented_content(&definition),
            "expected unsupported markers in the definition debug output to be rejected"
        );
    }

    #[cfg(feature = "generated-registry")]
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn try_compile_card_accepts_generated_supported_definitions() {
        let definition = CardRegistry::try_compile_card("Sicarian Infiltrator")
            .expect("supported generated definition should compile");
        assert_eq!(definition.name(), "Sicarian Infiltrator");
    }

    #[cfg(feature = "generated-registry")]
    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn try_compile_card_accepts_chromium_rampage_marker() {
        let definition = CardRegistry::try_compile_card("Chromium")
            .expect("Chromium's rampage marker should be backed by runtime semantics");

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );
    }

    #[test]
    fn reject_unsupported_generated_definition_returns_error() {
        let card = CardBuilder::new(CardId::new(), "Rejected Fallback")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::unsupported_parser_line(
            "reject me",
            "ParseError(\"mock\")",
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        let error = reject_unsupported_generated_definition(definition)
            .expect_err("unsupported generated fallback should be rejected");
        assert!(
            error.to_ascii_lowercase().contains("unsupported"),
            "expected unsupported compile error, got {error}"
        );
    }

    #[test]
    fn generated_definition_support_rejects_placeholder_static_abilities() {
        let card = CardBuilder::new(CardId::new(), "Custom Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let custom =
            Ability::static_ability(StaticAbility::rule_fallback_text("Probe custom rule text"));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(custom);

        assert!(!generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_rampage_marker_with_runtime_semantics() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Rampage Probe")
            .card_types(vec![CardType::Creature])
            .rampage(2)
            .build();

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );
    }

    #[test]
    fn generated_definition_support_rejects_keyword_fallback_text() {
        let card = CardBuilder::new(CardId::new(), "Unsupported Rampage Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(Ability::static_ability(
            StaticAbility::keyword_fallback_text("rampage 2"),
        ));

        let message = generated_definition_unsupported_mechanics_message(&definition)
            .expect("keyword fallback text should still be reported");
        assert!(
            message.contains("unsupported keyword marker: rampage 2"),
            "expected unsupported keyword marker message, got {message}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_prowess() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Prowess Probe")
            .parse_text("Prowess")
            .expect("prowess parse should succeed");

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_cipher() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Cipher Probe")
            .parse_text("Draw a card.\nCipher")
            .expect("cipher parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_split_second() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Split Second Probe")
            .parse_text("Split second\nDraw a card.")
            .expect("split second parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_riot() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Riot Probe")
            .parse_text("Riot")
            .expect("riot parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_unleash() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Unleash Probe")
            .parse_text("Unleash")
            .expect("unleash parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_unearth() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Unearth Probe")
            .parse_text(
                "Mana cost: {1}{B}\nType: Creature — Zombie\nPower/Toughness: 2/1\nUnearth {2}{B}",
            )
            .expect("unearth parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_outlast() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Outlast Probe")
            .parse_text(
                "Mana cost: {W}\nType: Creature — Human Soldier\nPower/Toughness: 1/1\nOutlast {W}",
            )
            .expect("outlast parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_vanishing() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Vanishing Probe")
            .parse_text(
                "Mana cost: {2}{U}\nType: Creature — Illusion\nPower/Toughness: 2/2\nVanishing 3",
            )
            .expect("vanishing parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_devour() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Devour Probe")
            .parse_text("Mana cost: {4}{R}\nType: Creature — Beast\nPower/Toughness: 2/2\nDevour 2")
            .expect("devour parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_buyback() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Buyback Probe")
            .parse_text("Mana cost: {1}{U}\nType: Instant\nBuyback {3}\nDraw a card.")
            .expect("buyback parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_bloodthirst() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Bloodthirst Probe")
            .parse_text(
                "Mana cost: {6}{G}\nType: Creature — Wurm\nPower/Toughness: 6/6\nBloodthirst 3",
            )
            .expect("bloodthirst parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_ward_pay_life() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Ward Pay Life Probe")
            .parse_text(
                "Mana cost: {2}{B}\nType: Creature — Horror\nPower/Toughness: 2/2\nWard—Pay 3 life.",
            )
            .expect("ward pay-life parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_bolster() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Bolster Probe")
            .parse_text(
                "Mana cost: {3}{W}\nType: Creature — Human Soldier\nPower/Toughness: 2/2\nWhen this creature enters, bolster 2.",
            )
            .expect("bolster parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_rebound() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Rebound Probe")
            .parse_text("Mana cost: {1}{U}\nType: Instant\nGain 1 life.\nRebound")
            .expect("rebound parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_parsed_cascade() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Cascade Probe")
            .parse_text("Mana cost: {2}{R}\nType: Sorcery\nDraw a card.\nCascade")
            .expect("cascade parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_saruman_of_many_colors() {
        let text = "Ward—Discard an enchantment, instant, or sorcery card.\nWhenever you cast your second spell each turn, each opponent mills two cards. When one or more cards are milled this way, exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard. Copy the exiled card. You may cast the copy without paying its mana cost.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Saruman of Many Colors")
            .parse_text(text)
            .expect("saruman parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_good_day_to_pie() {
        let text = "Tap up to two target creatures.\nWhenever you put a name sticker on a creature, you may return this card from your graveyard to your hand.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A Good Day to Pie")
            .parse_text(text)
            .expect("a good day to pie parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_asari_captain() {
        let text = "Trample, haste\nWhenever a Samurai or Warrior you control attacks alone, it gets +1/+0 until end of turn for each Samurai or Warrior you control.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Asari Captain")
            .parse_text(text)
            .expect("a-asari captain parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_brine_comber() {
        let text = "Mana cost: {1}{W}{U}\nType: Creature — Spirit // Enchantment — Aura\nPower/Toughness: 2/2\nWhenever this creature enters or becomes the target of an Aura spell, create a 1/1 white Spirit creature token with flying.\nDisturb {W}{U} (You may cast this card from your graveyard transformed for its disturb cost.)";
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "A-Brine Comber // A-Brinebound Gift")
                .parse_text(text)
                .expect("a-brine comber parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_devoted_grafkeeper() {
        let text = "Mana cost: {W}{U}\nType: Creature — Human Peasant // Creature — Spirit\nPower/Toughness: 2/2\nWhen Devoted Grafkeeper enters, mill four cards.\nWhenever you cast a spell from your graveyard, tap target creature you don't control.\nDisturb {1}{W}{U} (You may cast this card from your graveyard transformed for its disturb cost.)";
        let definition = CardDefinitionBuilder::new(
            CardId::new(),
            "A-Devoted Grafkeeper // A-Departed Soulkeeper",
        )
        .parse_text(text)
        .expect("a-devoted grafkeeper parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_dokuchi_silencer() {
        let text = "Mana cost: {1}{B}\nType: Creature — Human Ninja\nPower/Toughness: 2/1\nNinjutsu {1}{B} ({1}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever Dokuchi Silencer deals combat damage to a player, you may discard a card. When you do, destroy target creature or planeswalker that player controls.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Dokuchi Silencer")
            .parse_text(text)
            .expect("a-dokuchi silencer parse should succeed");

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_covert_technician() {
        let text = "Mana cost: {2}{U}\nType: Creature\nPower/Toughness: 2/4\nNinjutsu {1}{U} ({1}{U}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever Covert Technician deals combat damage to a player, you may put an artifact card with mana value less than or equal to that damage from your hand onto the battlefield.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Covert Technician")
            .parse_text(text)
            .expect("covert technician parse should succeed");

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
        assert!(
            debug.contains("lessthanorequalexpr")
                && debug.contains("eventvalue(")
                && debug.contains("amount"),
            "Covert Technician should keep the dynamic 'that damage' mana value gate, got {debug}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_glamdring() {
        let text = "Mana cost: {2}\nType: Legendary Artifact\nEquipped creature has first strike and gets +1/+0 for each instant and sorcery card in your graveyard.\nWhenever equipped creature deals combat damage to a player, you may cast an instant or sorcery spell from your hand with mana value less than or equal to that damage without paying its mana cost.\nEquip {3}";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Glamdring")
            .parse_text(text)
            .expect("glamdring parse should succeed");

        assert!(
            generated_definition_is_supported(&definition),
            "{:?}\n{definition:#?}",
            generated_definition_support_issues(&definition)
        );

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
        assert!(
            debug.contains("maycastmatchingspellwithoutpayingmanacost")
                && debug.contains("eventvalue")
                && debug.contains("amount"),
            "Glamdring should keep the dynamic 'that damage' mana value gate, got {debug}"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_deepcavern_imp() {
        let text = "Mana cost: {2}{B}\nType: Creature — Imp Rebel\nPower/Toughness: 2/2\nFlying, haste\nEcho—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Deepcavern Imp")
            .parse_text(text)
            .expect("deepcavern imp parse should succeed");

        assert!(generated_definition_is_supported(&definition));

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_metropolis_angel() {
        let text = "Mana cost: {3}{W}{U}\nType: Creature — Angel Soldier\nPower/Toughness: 3/3\nFlying\nWhenever you attack with one or more creatures with counters on them, draw a card.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Metropolis Angel")
            .parse_text(text)
            .expect("a-metropolis angel parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_nadu_winged_wisdom() {
        let text = "Mana cost: {1}{G}{U}\nType: Legendary Creature — Bird Wizard\nPower/Toughness: 3/4\nFlying\nWhenever a creature you control becomes the target of a spell or ability, reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand. This ability triggers only twice each turn.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Nadu, Winged Wisdom")
            .parse_text(text)
            .expect("a-nadu winged wisdom parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_radha_coalition_warlord() {
        let text = "Mana cost: {1}{R}{G}\nType: Legendary Creature — Elf Warrior\nPower/Toughness: 3/3\nDomain — Whenever Radha, Coalition Warlord enters or becomes tapped, another target creature you control gets +X/+X until end of turn, where X is the number of basic land types among lands you control.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Radha, Coalition Warlord")
            .parse_text(text)
            .expect("a-radha coalition warlord parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_rockslide_sorcerer() {
        let text = "Mana cost: {2}{R}\nType: Creature — Human Wizard\nPower/Toughness: 2/2\nWhenever you cast an instant, sorcery, or Wizard spell, Rockslide Sorcerer deals 1 damage to any target.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Rockslide Sorcerer")
            .parse_text(text)
            .expect("a-rockslide sorcerer parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_shipwreck_sifters() {
        let text = "Mana cost: {1}{U}\nType: Creature — Spirit\nPower/Toughness: 1/2\nWhen Shipwreck Sifters enters, draw a card, then discard a card.\nWhenever a Spirit card or a card with disturb is put into your graveyard from anywhere, put a +1/+1 counter on Shipwreck Sifters.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Shipwreck Sifters")
            .parse_text(text)
            .expect("a-shipwreck sifters parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_symmetry_sage() {
        let text = "Mana cost: {U}\nType: Creature — Human Wizard\nPower/Toughness: 0/3\nFlying\nMagecraft — Whenever you cast or copy an instant or sorcery spell, target creature you control has base power 3 until end of turn.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Symmetry Sage")
            .parse_text(text)
            .expect("a-symmetry sage parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn generated_definition_support_accepts_a_vampire_scrivener() {
        let text = "Mana cost: {3}{B}\nType: Creature — Vampire Warlock\nPower/Toughness: 2/2\nFlying\nWhenever you gain life during your turn, put a +1/+1 counter on Vampire Scrivener.\nWhenever you lose life during your turn, put a +1/+1 counter on Vampire Scrivener.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Vampire Scrivener")
            .parse_text(text)
            .expect("a-vampire scrivener parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn registry_skips_parser_fallback_definitions() {
        let card = CardBuilder::new(CardId::new(), "Skipped Fallback")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::unsupported_parser_line(
            "skip me",
            "ParseError(\"mock\")",
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        let mut registry = CardRegistry::new();
        registry.register(definition);

        assert_eq!(registry.len(), 0);
        assert!(registry.get("Skipped Fallback").is_none());
    }
}
