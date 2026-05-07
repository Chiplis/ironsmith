import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/ConniveTest.java",
  "tests": [
    {
      "name": "test_OnLandDiscard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hypnotic Grifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Plains"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hypnotic Grifter",
          "power": 1,
          "toughness": 2
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Plains",
          "count": 1
        }
      ]
    },
    {
      "name": "test_OnCreatureDiscard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hypnotic Grifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ledger Shredder",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Ledger Shredder"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hypnotic Grifter",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Ledger Shredder",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Madness",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hypnotic Grifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Alms of the Vein",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{3}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Alms of the Vein"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Yes"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Hypnotic Grifter",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Alms of the Vein",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "test_OpponentCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Obscura Confluence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Plains",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Crazed Goblin",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Obscura Confluence"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Crazed Goblin"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Crazed Goblin"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Crazed Goblin"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Plains"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Plains"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Plains"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Crazed Goblin",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Plains",
          "count": 3
        }
      ]
    },
    {
      "name": "test_LKI_PsychicPickpocket",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Psychic Pickpocket",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Psychic Pickpocket"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Lightning Bolt",
          "target": "Psychic Pickpocket"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Swamp"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Sol Ring"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Psychic Pickpocket",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Lightning Bolt",
          "count": 1
        }
      ]
    },
    {
      "name": "test_LKI_Source_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Raffine's Silencer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Raffine's Silencer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Raffine's Silencer",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_LKI_Source_Dies",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Raffine's Silencer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Raffine's Silencer"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Raffine's Silencer"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Raffine's Silencer",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_LKI_SpymastersVault_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spymaster's Vault",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerA)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Razorclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{B}, {T}: Target creature",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_LKI_SpymastersVault_Fizzle",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spymaster's Vault",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerA)"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Razorclaw Bear"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{B}, {T}: Target creature",
          "target": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        }
      ]
    },
    {
      "name": "test_LKI_ChangeOfPlans_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Change of Plans",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Change of Plans"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    },
    {
      "name": "test_LKI_ChangeOfPlans_Fizzle",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Change of Plans",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerA)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Change of Plans"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        }
      ]
    },
    {
      "name": "test_LKI_ChangeOfPlans_MultipleTargets",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Change of Plans",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCustomEffect_TargetDestroy(playerA)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Change of Plans"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=2"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion^Razorclaw Bear"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "target destroy",
          "target": "Silvercoat Lion"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Razorclaw Bear",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        }
      ]
    }
  ]
});
