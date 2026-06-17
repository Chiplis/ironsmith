//! Registry-owned runtime card-loading implementation shared with runtime.

use super::{generated_meld_counterparts, generated_registry};
#[cfg(all(feature = "handwritten-parse-support", not(test)))]
use crate::ability::Ability;
use crate::ability::AbilityKind;
use crate::cards::CardDefinition;
#[cfg(all(feature = "handwritten-parse-support", not(test)))]
use crate::cards::CardDefinitionBuilder;
#[cfg(all(feature = "handwritten-parse-support", not(test)))]
use crate::cards::definitions::*;
use crate::ids::CardId;
use crate::static_abilities::StaticAbilityId;
use std::collections::HashMap;
#[cfg(any(test, feature = "handwritten-parse-support"))]
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

/// Registry of all card definitions.
///
/// Provides lookup by name and other queries.
#[derive(Debug, Clone, Default)]
pub struct CardRegistry {
    /// Cards indexed by name
    cards: HashMap<String, CardDefinition>,
    /// Mapping for looking up cards by CardId without duplicating CardDefinition storage.
    names_by_id: HashMap<CardId, String>,
    /// Alias name -> canonical name (used for card-face layouts where Scryfall's
    /// `name` is "Front // Back" but the playable card name is the front face).
    aliases: HashMap<String, String>,
}

impl CardRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            names_by_id: HashMap::new(),
            aliases: HashMap::new(),
        }
    }

    /// Create a card registry.
    ///
    /// This loads generated parser cards first and then overlays handwritten definitions.
    /// The handwritten registry is used as an override layer for cards that need custom
    /// metadata or bespoke parsing behavior.
    pub fn with_builtin_cards() -> Self {
        let mut registry = Self::new();
        // Non-test builds are populated from the registry DB via generated parser output.
        generated_registry::register_generated_parser_cards(&mut registry);
        #[cfg(all(feature = "handwritten-parse-support", not(test)))]
        registry.register_builtin_handwritten_cards_if(|_| true);
        #[cfg(test)]
        super::register_builtin_handwritten_cards_if_for_runtime_tests(&mut registry, |_| true);

        registry
    }

    /// Ensure cards with any of the requested names are loaded into this registry.
    ///
    /// Matching is case-insensitive and ignores surrounding whitespace.
    pub fn ensure_cards_loaded<'a>(&mut self, names: impl IntoIterator<Item = &'a str>) {
        let requested_names = names.into_iter().collect::<Vec<_>>();
        if requested_names.is_empty() {
            return;
        }

        let unresolved_names = requested_names
            .iter()
            .map(|name| name.trim())
            .filter(|name| !name.is_empty() && self.get(name).is_none())
            .collect::<Vec<_>>();
        if unresolved_names.is_empty() {
            self.ensure_requested_linked_faces_loaded(&requested_names);
            return;
        }

        let requested_name_keys = unresolved_names
            .iter()
            .map(|name| normalize_card_lookup_name(name))
            .collect::<std::collections::HashSet<_>>();
        let requested_loose_name_keys = unresolved_names
            .iter()
            .map(|name| normalize_card_loose_lookup_name(name))
            .collect::<std::collections::HashSet<_>>();

        generated_registry::register_generated_parser_cards_if_name(self, |name| {
            requested_name_keys.contains(&normalize_card_lookup_name(name))
                || requested_loose_name_keys.contains(&normalize_card_loose_lookup_name(name))
        });

        for requested in &unresolved_names {
            let normalized = requested.trim();
            if normalized.is_empty() || self.get(normalized).is_some() {
                continue;
            }

            let loose_key = normalize_card_loose_lookup_name(normalized);
            if let Some(canonical) = self
                .cards
                .keys()
                .find(|name| normalize_card_loose_lookup_name(name) == loose_key)
                .cloned()
            {
                self.register_alias(normalized, canonical);
                continue;
            }

            let Some((resolved_name, _parse_block)) =
                generated_registry::generated_parser_card_parse_source(normalized)
            else {
                continue;
            };

            let resolved_name_key = normalize_card_lookup_name(&resolved_name);
            generated_registry::register_generated_parser_cards_if_name(self, |name| {
                normalize_card_lookup_name(name) == resolved_name_key
            });
            if self.get(&resolved_name).is_some() {
                if !resolved_name.eq_ignore_ascii_case(normalized) {
                    self.register_alias(normalized, &resolved_name);
                }
                continue;
            }

            if let Ok(definition) = generated_registry::try_compile_card_by_name(&resolved_name) {
                self.register(definition);
                if self.get(&resolved_name).is_some()
                    && !resolved_name.eq_ignore_ascii_case(normalized)
                {
                    self.register_alias(normalized, &resolved_name);
                }
                continue;
            }

            #[cfg(all(feature = "handwritten-parse-support", not(test)))]
            {
                let Ok(definition) =
                    compile_generated_parser_card_allow_unsupported(&resolved_name, &_parse_block)
                else {
                    continue;
                };

                if !resolved_name.eq_ignore_ascii_case(normalized) {
                    // Flavor/printed aliases should still resolve to their canonical card even if the
                    // canonical generated definition currently needs the unsupported fallback marker.
                    // We keep that fallback visible on the definition rather than pretending support.
                    self.register_explicit(definition);
                    self.register_alias(normalized, &resolved_name);
                    continue;
                }

                self.register(definition);
                if self.get(&resolved_name).is_some() {
                    self.register_alias(normalized, &resolved_name);
                }
            }
        }

        // Prefer handwritten definitions for overlapping cards and provide
        // fallbacks for cards whose generated parser definition is unavailable.
        #[cfg(all(feature = "handwritten-parse-support", not(test)))]
        {
            let requested_keys = requested_names
                .iter()
                .map(|name| normalize_card_constructor_key(name))
                .collect::<std::collections::HashSet<_>>();
            self.register_builtin_handwritten_cards_if(|constructor_key| {
                requested_keys.contains(constructor_key)
                    || constructor_key
                        .strip_prefix("basic_")
                        .is_some_and(|stripped| requested_keys.contains(stripped))
            });
        }
        #[cfg(test)]
        {
            let requested_keys = requested_names
                .iter()
                .map(|name| normalize_card_constructor_key(name))
                .collect::<std::collections::HashSet<_>>();
            super::register_builtin_handwritten_cards_if_for_runtime_tests(
                self,
                |constructor_key| {
                    requested_keys.contains(constructor_key)
                        || constructor_key
                            .strip_prefix("basic_")
                            .is_some_and(|stripped| requested_keys.contains(stripped))
                },
            );
        }

        self.ensure_requested_linked_faces_loaded(&requested_names);
    }

    fn ensure_requested_linked_faces_loaded(&mut self, requested_names: &[&str]) {
        let mut linked_names = Vec::new();
        for requested in requested_names {
            let trimmed = requested.trim();
            if trimmed.is_empty() {
                continue;
            }

            if let Some(definition) = self.get(trimmed) {
                push_missing_linked_face_name(self, &mut linked_names, definition);
            }

            for face_name in trimmed.split("//").map(str::trim) {
                if face_name.is_empty() || face_name == trimmed {
                    continue;
                }
                if self.get(face_name).is_none() {
                    push_unique(&mut linked_names, face_name);
                } else if let Some(definition) = self.get(face_name) {
                    push_missing_linked_face_name(self, &mut linked_names, definition);
                }
            }
        }

        let missing = linked_names
            .into_iter()
            .filter(|name| self.get(name).is_none())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            self.ensure_cards_loaded(missing.iter().map(String::as_str));
        }
    }

    /// Ensure every generated parser definition is loaded into this registry.
    pub fn ensure_all_generated_cards_loaded(&mut self) {
        #[cfg(test)]
        {
            generated_registry::register_generated_parser_cards(self);
        }
        #[cfg(not(test))]
        {
            generated_registry::register_generated_parser_cards(self);
        }
    }

    /// Number of generated registry parse entries available for chunked preload.
    pub fn generated_parser_entry_count() -> usize {
        let _ = generated_registry::GENERATED_PARSER_CARD_SOURCE_COUNT;
        generated_registry::generated_parser_entry_count()
    }

    /// Generated parser card names without forcing all definitions to parse/register.
    pub fn generated_parser_card_names() -> Vec<String> {
        generated_registry::generated_parser_card_names()
    }

    /// Names of cards currently supported by the registry implementation.
    pub fn supported_card_names() -> Vec<String> {
        let mut registry = Self::with_builtin_cards();
        registry.ensure_all_generated_cards_loaded();
        let mut names = registry.cards.keys().cloned().collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Semantic fidelity score for a generated parser card name.
    pub fn generated_parser_semantic_score(name: &str) -> Option<f32> {
        generated_registry::generated_parser_semantic_score(name)
    }

    /// Source parse block for a generated parser card name.
    pub fn generated_parser_card_parse_source(name: &str) -> Option<(String, String)> {
        generated_registry::generated_parser_card_parse_source(name)
    }

    /// Precomputed counts of cards meeting each integer threshold from 1%..=100%.
    pub fn generated_parser_semantic_threshold_counts() -> [usize; 100] {
        generated_registry::generated_parser_semantic_threshold_counts()
    }

    /// Number of generated parser card names that have an embedded semantic score.
    pub fn generated_parser_semantic_scored_count() -> usize {
        generated_registry::generated_parser_semantic_scored_count()
    }

    /// Incrementally parse/register generated cards and return the next cursor position.
    pub fn preload_generated_cards_chunk(&mut self, cursor: usize, chunk_size: usize) -> usize {
        generated_registry::register_generated_parser_cards_chunk(self, cursor, chunk_size)
    }

    /// Try to compile a card by name, returning the specific error if it fails.
    ///
    /// Used to distinguish "card not in database" from "card exists but failed to compile".
    pub fn try_compile_card(name: &str) -> Result<CardDefinition, String> {
        if let Some(definition) = try_compile_builtin_card_by_name(name) {
            return Ok(definition);
        }

        let definition = generated_registry::try_compile_card_by_name(name)?;
        reject_unsupported_generated_definition(definition)
    }

    /// Create a card registry with only the requested hand-written cards plus generated parser cards.
    #[cfg(test)]
    pub fn with_builtin_cards_for_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let mut registry = Self::new();
        registry.ensure_cards_loaded(names);
        registry
    }

    #[cfg(all(feature = "handwritten-parse-support", not(test)))]
    fn register_builtin_handwritten_cards_if<F>(&mut self, mut include_constructor_key: F)
    where
        F: FnMut(&str) -> bool,
    {
        macro_rules! maybe_register {
            ($constructor:ident) => {
                if include_constructor_key(stringify!($constructor)) {
                    self.register($constructor());
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
        maybe_register!(preordain);
        maybe_register!(barrier_of_bones);
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

    /// Register a card definition.
    pub fn register(&mut self, def: CardDefinition) {
        if !generated_definition_is_supported(&def) {
            return;
        }
        self.register_explicit(def);
    }

    fn register_explicit(&mut self, def: CardDefinition) {
        let name = def.card.name.clone();
        self.names_by_id
            .entry(def.card.id)
            .or_insert_with(|| name.clone());
        self.cards.insert(name, def);
    }

    /// Look up a card by name.
    pub fn get(&self, name: &str) -> Option<&CardDefinition> {
        if let Some(def) = self.cards.get(name) {
            return Some(def);
        }
        let canonical = self
            .aliases
            .get(name)
            .or_else(|| self.aliases.get(&normalize_card_lookup_name(name)))?;
        self.cards.get(canonical)
    }

    /// Register an alternate name for an existing definition.
    pub fn register_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        let alias = alias.into();
        let canonical = canonical.into();
        self.aliases.insert(alias.clone(), canonical.clone());

        let normalized = normalize_card_lookup_name(&alias);
        if !normalized.is_empty() && normalized != alias {
            self.aliases.insert(normalized, canonical);
        }
    }

    /// Look up a card by CardId.
    pub fn get_by_id(&self, id: CardId) -> Option<&CardDefinition> {
        let name = self.names_by_id.get(&id)?;
        self.cards.get(name)
    }

    pub fn linked_face_definition_by_name_or_id(
        &self,
        face_name: Option<&str>,
        id: Option<CardId>,
    ) -> Option<&CardDefinition> {
        if let Some(face_name) = face_name {
            if let Some(definition) = self.get(face_name) {
                return Some(definition);
            }
            if let Some(definition) = loose_name_match(self, face_name) {
                return Some(definition);
            }
        }

        id.and_then(|card_id| self.get_by_id(card_id))
    }

    /// Get all card definitions.
    pub fn all(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values()
    }

    /// Get the number of registered cards.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Get all creatures.
    pub fn creatures(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values().filter(|c| c.is_creature())
    }

    /// Get all spells (instants and sorceries).
    pub fn spells(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values().filter(|c| c.is_spell())
    }

    /// Get all lands.
    pub fn lands(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values().filter(|c| c.card.is_land())
    }
}

#[cfg(all(feature = "handwritten-parse-support", not(test)))]
fn compile_generated_parser_card_allow_unsupported(
    name: &str,
    parse_block: &str,
) -> Result<CardDefinition, String> {
    let builder = CardDefinitionBuilder::new(CardId::new(), name);
    match builder.parse_text_allow_unsupported(parse_block.to_string()) {
        Ok(definition) => Ok(definition),
        Err(err) => {
            let mut definition = CardDefinitionBuilder::new(CardId::new(), name)
                .oracle_text(parse_block.to_string())
                .build();
            definition.abilities.push(Ability::static_ability(
                crate::static_abilities::StaticAbility::unsupported_parser_line(
                    parse_block,
                    format!("{err:?}"),
                ),
            ));
            Ok(definition)
        }
    }
}

pub fn unsupported_generated_definition_error(definition: &CardDefinition) -> Option<String> {
    if !generated_definition_has_unimplemented_content(definition) {
        return None;
    }

    Some(
        generated_definition_unsupported_mechanics_message(definition).unwrap_or_else(|| {
            format!(
                "Card compiled but contains unsupported mechanics: {}",
                definition.name()
            )
        }),
    )
}

pub fn reject_unsupported_generated_definition(
    definition: CardDefinition,
) -> Result<CardDefinition, String> {
    if let Some(error) = unsupported_generated_definition_error(&definition) {
        return Err(error);
    }

    Ok(definition)
}

#[cfg(all(feature = "handwritten-parse-support", not(test)))]
fn try_compile_builtin_card_by_name(name: &str) -> Option<CardDefinition> {
    let requested_keys = requested_constructor_keys(name);
    if requested_keys.is_empty() {
        return None;
    }

    let mut registry = CardRegistry::new();
    registry.register_builtin_handwritten_cards_if(|constructor_key| {
        constructor_key_matches_any_request(constructor_key, &requested_keys)
    });

    registry
        .get(name)
        .cloned()
        .or_else(|| loose_name_match(&registry, name).cloned())
        .or_else(|| first_face_lookup(&registry, name).cloned())
}

#[cfg(test)]
fn try_compile_builtin_card_by_name(name: &str) -> Option<CardDefinition> {
    let requested_keys = requested_constructor_keys(name);
    if requested_keys.is_empty() {
        return None;
    }

    let mut registry = CardRegistry::new();
    super::register_builtin_handwritten_cards_if_for_runtime_tests(
        &mut registry,
        |constructor_key| constructor_key_matches_any_request(constructor_key, &requested_keys),
    );

    registry
        .get(name)
        .cloned()
        .or_else(|| loose_name_match(&registry, name).cloned())
        .or_else(|| first_face_lookup(&registry, name).cloned())
}

#[cfg(not(any(test, feature = "handwritten-parse-support")))]
fn try_compile_builtin_card_by_name(_name: &str) -> Option<CardDefinition> {
    None
}

#[cfg(any(test, feature = "handwritten-parse-support"))]
fn normalize_card_constructor_key(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut previous_was_separator = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if ch == '\'' {
            // Keep possessive words aligned with constructor names:
            // "Akroma's Will" -> "akromas_will".
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }

    normalized.trim_matches('_').to_string()
}

#[cfg(any(test, feature = "handwritten-parse-support"))]
fn requested_constructor_keys(name: &str) -> HashSet<String> {
    let mut keys = HashSet::new();
    let full_key = normalize_card_constructor_key(name);
    if !full_key.is_empty() {
        keys.insert(full_key);
    }

    for face in name.split("//") {
        let face_key = normalize_card_constructor_key(face);
        if !face_key.is_empty() {
            keys.insert(face_key);
        }
    }

    keys
}

#[cfg(any(test, feature = "handwritten-parse-support"))]
fn constructor_key_matches_any_request(
    constructor_key: &str,
    requested_keys: &HashSet<String>,
) -> bool {
    requested_keys.contains(constructor_key)
        || constructor_key
            .strip_prefix("basic_")
            .is_some_and(|stripped| requested_keys.contains(stripped))
}

fn normalize_card_lookup_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn normalize_card_loose_lookup_name(name: &str) -> String {
    let mut normalized = String::new();
    for ch in name.chars() {
        match ch {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' | 'Á' | 'À' | 'Â' | 'Ä' | 'Ã' | 'Å' => {
                normalized.push('a')
            }
            'é' | 'è' | 'ê' | 'ë' | 'É' | 'È' | 'Ê' | 'Ë' => normalized.push('e'),
            'í' | 'ì' | 'î' | 'ï' | 'Í' | 'Ì' | 'Î' | 'Ï' => normalized.push('i'),
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'Ó' | 'Ò' | 'Ô' | 'Ö' | 'Õ' => {
                normalized.push('o')
            }
            'ú' | 'ù' | 'û' | 'ü' | 'Ú' | 'Ù' | 'Û' | 'Ü' => normalized.push('u'),
            'ñ' | 'Ñ' => normalized.push('n'),
            'ç' | 'Ç' => normalized.push('c'),
            _ if ch.is_ascii_alphanumeric() => normalized.push(ch.to_ascii_lowercase()),
            _ => {}
        }
    }
    normalized
}

fn loose_name_match<'a>(registry: &'a CardRegistry, requested: &str) -> Option<&'a CardDefinition> {
    let requested_key = normalize_card_loose_lookup_name(requested);
    if requested_key.is_empty() {
        return None;
    }

    registry.cards.iter().find_map(|(name, definition)| {
        (normalize_card_loose_lookup_name(name) == requested_key).then_some(definition)
    })
}

fn push_missing_linked_face_name(
    registry: &CardRegistry,
    linked_names: &mut Vec<String>,
    definition: &CardDefinition,
) {
    let Some(face_name) = definition.card.other_face_name.as_deref() else {
        return;
    };
    if registry.get(face_name).is_none() {
        push_unique(linked_names, face_name);
    }
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if values.iter().any(|existing| existing == value) {
        return;
    }
    values.push(value.to_string());
}

#[cfg(any(test, feature = "handwritten-parse-support"))]
fn first_face_lookup<'a>(
    registry: &'a CardRegistry,
    requested: &str,
) -> Option<&'a CardDefinition> {
    let (front_face, _) = requested.split_once("//")?;
    let front_face = front_face.trim();
    if front_face.is_empty() {
        return None;
    }

    registry
        .get(front_face)
        .or_else(|| loose_name_match(registry, front_face))
}

/// A lazily-constructed singleton registry for effect/runtime lookups.
///
/// Most engine logic avoids needing the registry at runtime, but mechanics like
/// flip cards need to resolve the other face's definition.
pub fn builtin_registry() -> &'static CardRegistry {
    static REGISTRY: OnceLock<CardRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CardRegistry::with_builtin_cards)
}

fn runtime_custom_registry() -> &'static Mutex<CardRegistry> {
    static REGISTRY: OnceLock<Mutex<CardRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(CardRegistry::new()))
}

pub fn clear_runtime_custom_cards() {
    if let Ok(mut registry) = runtime_custom_registry().lock() {
        *registry = CardRegistry::new();
    }
}

pub fn register_runtime_custom_card(definition: CardDefinition) {
    if let Ok(mut registry) = runtime_custom_registry().lock() {
        registry.register(definition);
    }
}

pub fn linked_face_definition_by_name_or_id(
    name: Option<&str>,
    id: Option<CardId>,
) -> Option<CardDefinition> {
    if let Ok(registry) = runtime_custom_registry().lock() {
        if let Some(card_id) = id
            && let Some(definition) = registry.get_by_id(card_id).cloned()
        {
            return Some(definition);
        }

        if let Some(face_name) = name
            && let Some(definition) = registry.get(face_name).cloned()
        {
            return Some(definition);
        }

        if let Some(face_name) = name
            && let Some(definition) = loose_name_match(&registry, face_name).cloned()
        {
            return Some(definition);
        }
    }

    if let Some(name) = name {
        if let Ok(definition) = CardRegistry::try_compile_card(name) {
            return Some(definition);
        }

        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded([name]);
        if let Some(definition) = registry.get(name).cloned() {
            return Some(definition);
        }
    }

    let card_id = id?;
    let registry = CardRegistry::with_builtin_cards();
    registry.get_by_id(card_id).cloned()
}

pub fn meld_counterpart_name(name: &str) -> Option<&'static str> {
    generated_meld_counterparts::GENERATED_MELD_COUNTERPARTS
        .iter()
        .find_map(|(candidate, counterpart)| {
            candidate.eq_ignore_ascii_case(name).then_some(*counterpart)
        })
}

const UNSUPPORTED_PARSER_LINE_FALLBACK_PREFIX: &str = "Unsupported parser line fallback:";

const GENERATED_SUPPORT_ISSUE_MAX_LEN: usize = 180;

fn truncate_generated_support_issue(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.chars().count() <= GENERATED_SUPPORT_ISSUE_MAX_LEN {
        return trimmed.to_string();
    }
    let mut out = String::with_capacity(GENERATED_SUPPORT_ISSUE_MAX_LEN + 3);
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= GENERATED_SUPPORT_ISSUE_MAX_LEN {
            break;
        }
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn compact_generated_support_text(raw: &str) -> String {
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    truncate_generated_support_issue(&compact)
}

fn extract_fallback_reason(display: &str) -> String {
    let body = display
        .strip_prefix(UNSUPPORTED_PARSER_LINE_FALLBACK_PREFIX)
        .map(str::trim)
        .unwrap_or_else(|| display.trim());

    if let Some(start) = body.find("ParseError(\"") {
        let remainder = &body[start + "ParseError(\"".len()..];
        if let Some(end) = remainder.rfind("\")") {
            return compact_generated_support_text(&remainder[..end]);
        }
        if let Some(end) = remainder.rfind('"') {
            return compact_generated_support_text(&remainder[..end]);
        }
    }

    if let Some(start) = body.find("ParseError(") {
        let remainder = &body[start + "ParseError(".len()..];
        let reason = remainder.strip_suffix(')').unwrap_or(remainder);
        return compact_generated_support_text(reason);
    }

    if let Some((_, reason_part)) = body.rsplit_once(" (") {
        let reason = reason_part.strip_suffix(')').unwrap_or(reason_part).trim();
        if let Some(inner) = reason
            .strip_prefix("ParseError(\"")
            .and_then(|value| value.strip_suffix("\")"))
        {
            return compact_generated_support_text(inner);
        }
        return compact_generated_support_text(reason);
    }

    compact_generated_support_text(body)
}

fn raw_runtime_marker_issue(definition: &CardDefinition) -> Option<String> {
    let raw_debug = format!("{definition:#?}");
    let raw_lower = raw_debug.to_ascii_lowercase();
    let marker_index = ["unimplemented", "unsupported"]
        .iter()
        .filter_map(|needle| raw_lower.find(needle))
        .min()?;
    let line_start = raw_debug[..marker_index]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let line_end = raw_debug[marker_index..]
        .find('\n')
        .map(|index| marker_index + index)
        .unwrap_or(raw_debug.len());
    let marker_line = raw_debug[line_start..line_end].trim();
    if marker_line.is_empty() {
        return None;
    }
    Some(format!(
        "runtime marker: {}",
        compact_generated_support_text(marker_line)
    ))
}

pub(crate) fn generated_definition_support_issues(definition: &CardDefinition) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();

    let mut push_issue = |label: &str, detail: String| {
        let detail = compact_generated_support_text(&detail);
        if detail.is_empty() {
            return;
        }
        let message = format!("{label}: {detail}");
        if !issues.iter().any(|existing| existing == &message) {
            issues.push(message);
        }
    };

    for ability in &definition.abilities {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            continue;
        };
        let display = static_ability.display();
        match static_ability.id() {
            StaticAbilityId::UnsupportedParserLine => {
                let reason = extract_fallback_reason(&display);
                if reason.is_empty() {
                    push_issue("unsupported parser fallback", display);
                } else {
                    push_issue("unsupported parser fallback", reason);
                }
            }
            StaticAbilityId::KeywordFallbackText => {
                if !display.to_ascii_lowercase().starts_with("craft with") {
                    push_issue("unsupported keyword marker", display);
                }
            }
            StaticAbilityId::RuleFallbackText => {
                push_issue("unsupported rules fallback", display);
            }
            _ => {}
        }
    }

    if issues.is_empty() && generated_definition_has_unimplemented_content(definition) {
        issues.push(
            raw_runtime_marker_issue(definition)
                .unwrap_or_else(|| "contains unimplemented runtime markers".to_string()),
        );
    }

    issues
}

pub fn generated_definition_unsupported_mechanics_message(
    definition: &CardDefinition,
) -> Option<String> {
    let issues = generated_definition_support_issues(definition);
    if issues.is_empty() {
        return None;
    }

    const MAX_ISSUES_IN_MESSAGE: usize = 3;
    let shown = issues
        .iter()
        .take(MAX_ISSUES_IN_MESSAGE)
        .cloned()
        .collect::<Vec<_>>();
    let mut details = shown.join(" | ");
    if issues.len() > MAX_ISSUES_IN_MESSAGE {
        details.push_str(&format!(
            " | (+{} more)",
            issues.len() - MAX_ISSUES_IN_MESSAGE
        ));
    }
    Some(format!(
        "Card compiled but contains unsupported mechanics: {details}"
    ))
}

/// Returns true if a parsed definition still contains unimplemented mechanics/effects.
///
/// This is used by generated registries and reporting utilities to keep support
/// classification consistent.
pub fn generated_definition_has_unimplemented_content(definition: &CardDefinition) -> bool {
    let has_placeholder_static = definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if matches!(
                    static_ability.id(),
                    StaticAbilityId::KeywordFallbackText
                        | StaticAbilityId::RuleFallbackText
                        | StaticAbilityId::UnsupportedParserLine
                )
        )
    });
    if has_placeholder_static {
        return true;
    }

    // Some parsed definitions still carry raw "unimplemented_*" internals
    // (for example, fallback custom triggers).
    let raw_debug = format!("{definition:#?}").to_ascii_lowercase();
    raw_debug.contains("unimplemented") || raw_debug.contains("unsupported")
}

/// Returns true when a generated parser definition can be safely included in the registry.
///
/// Generated wasm/demo registries should not include parser fallback placeholders that only
/// exist because unsupported mode swallowed a real parse failure.
pub(crate) fn generated_definition_is_supported(definition: &CardDefinition) -> bool {
    let has_parser_fallback_marker = definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::UnsupportedParserLine
                    && static_ability
                        .display()
                        .starts_with(UNSUPPORTED_PARSER_LINE_FALLBACK_PREFIX)
        )
    });

    if has_parser_fallback_marker {
        return false;
    }

    !generated_definition_has_unimplemented_content(definition)
}
