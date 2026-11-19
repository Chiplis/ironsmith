import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/modification/BattlefieldThaumaturgeTest.java",
  "tests": [
    {
      "name": "testSingleTargetReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Thaumaturge",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lightning Strike",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Akroan Skyguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Strike",
          "target": "Akroan Skyguard"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Akroan Skyguard",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Akroan Skyguard",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Mountain",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": false,
          "count": 1
        }
      ]
    },
    {
      "name": "testStriveTargetingReduction1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Thaumaturge",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Pharika's Chosen",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Silence the Believers",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Battlewise Hoplite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Silence the Believers",
          "target": "Pharika's Chosen^Battlewise Hoplite"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "name": "Pharika's Chosen",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "name": "Battlewise Hoplite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Pharika's Chosen",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Battlewise Hoplite",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pharika's Chosen",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Battlewise Hoplite",
          "count": 0
        },
        {
          "op": "assertTappedCount",
          "name": "Swamp",
          "tapped": true,
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 2
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": false,
          "count": 2
        }
      ]
    },
    {
      "name": "testStriveTargetingReduction2",
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
          "name": "Launch the Fleet",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Plains",
          "count": 1,
          "zone": "BATTLEFIELD"
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1,
          "zone": "BATTLEFIELD"
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Agent of Horizons",
          "count": 1,
          "zone": "BATTLEFIELD"
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Blood-Toll Harpy",
          "count": 1,
          "zone": "BATTLEFIELD"
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Anvilwrought Raptor",
          "count": 1,
          "zone": "BATTLEFIELD"
        },
        {
          "op": "addCard",
          "player": 0,
          "name": "Fleshmad Steed",
          "count": 1,
          "zone": "BATTLEFIELD"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Launch the Fleet",
          "target": {
            "groups": [
              [
                "Battlefield Thaumaturge",
                "Agent of Horizons",
                "Blood-Toll Harpy",
                "Anvilwrought Raptor",
                "Fleshmad Steed"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Launch the Fleet",
          "count": 1,
          "targetCount": 5
        },
        {
          "op": "assertStackObject",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "source": "Battlefield Thaumaturge",
          "count": 1,
          "abilityKind": "Triggered",
          "onTop": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "ability": "Hexproof",
          "expected": true
        },
        {
          "op": "waitStackResolved",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Battlefield Thaumaturge",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Agent of Horizons",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Blood-Toll Harpy",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Anvilwrought Raptor",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Fleshmad Steed",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 16
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 4
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Island",
          "tapped": false,
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Agent of Horizons",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Blood-Toll Harpy",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Anvilwrought Raptor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Fleshmad Steed",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 5
        }
      ]
    },
    {
      "name": "testVariableCostReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Curse of the Swine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "count": 1,
          "name": "Fleecemane Lion"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "count": 1,
          "name": "Sedge Scorpion"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "count": 1,
          "name": "Boon Satyr"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "count": 1,
          "name": "Bronze Sable"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=4"
        },
        {
          "op": "castSpell",
          "name": "Curse of the Swine",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": {
            "groups": [
              [
                "Fleecemane Lion",
                "Sedge Scorpion",
                "Boon Satyr",
                "Bronze Sable"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "source": "Curse of the Swine",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "xValue": 4,
          "targetCount": 4
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Fleecemane Lion",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Fleecemane Lion",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Fleecemane Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Sedge Scorpion",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Sedge Scorpion",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Sedge Scorpion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Boon Satyr",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Boon Satyr",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Boon Satyr",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bronze Sable",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Bronze Sable",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Bronze Sable",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Boar Token",
          "count": 4
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Island",
          "count": 2,
          "tapped": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Plains",
          "count": 4,
          "tapped": false
        }
      ]
    },
    {
      "name": "testMutipleTargetReduction",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
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
          "name": "Descent of the Dragons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sedge Scorpion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Boon Satyr",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Agent of Horizons",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Blood-Toll Harpy",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Anvilwrought Raptor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Descent of the Dragons",
          "target": {
            "groups": [
              [
                "Battlefield Thaumaturge",
                "Sedge Scorpion",
                "Boon Satyr",
                "Agent of Horizons",
                "Blood-Toll Harpy",
                "Anvilwrought Raptor"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Descent of the Dragons",
          "count": 1,
          "targetCount": 6
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Sedge Scorpion",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Boon Satyr",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Agent of Horizons",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Blood-Toll Harpy",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Anvilwrought Raptor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dragon Token",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Dragon Token",
          "count": 3
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Mountain",
          "count": 2,
          "tapped": true
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Swamp",
          "count": 4,
          "tapped": false
        }
      ]
    },
    {
      "name": "testTargetNonCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Fade into Antiquity",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Heliod, God of the Sun",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Fade into Antiquity",
          "target": "Heliod, God of the Sun"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Heliod, God of the Sun",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Heliod, God of the Sun",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "name": "Heliod, God of the Sun",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Forest",
          "tapped": true,
          "count": 3
        }
      ]
    },
    {
      "name": "testTargetWithAura",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Spectra Ward",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Spectra Ward",
          "target": "Battlefield Thaumaturge"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Spectra Ward",
          "count": 1
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 5
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Battlefield Thaumaturge",
          "ability": "Hexproof",
          "expected": true
        }
      ]
    }
  ]
});
