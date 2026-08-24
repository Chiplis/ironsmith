//! Compiler-owning registration of the legacy handwritten fixture catalog.
//!
//! The gameplay engine deliberately carries no parser-backed registry. This
//! compatibility catalog lives in the registry crate so products and tests
//! that need the handwritten fixtures can materialize them without creating an
//! engine-to-compiler dependency.

use super::CardRegistry;
use super::definitions::*;

pub fn register_builtin_handwritten_cards_if<F>(
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
