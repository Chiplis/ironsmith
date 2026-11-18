import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ncc/LethalSchemeTest.java",
  "tests": [
    {
      "name": "LethalSchemeNoConvoke",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPermanentCount",
          "player": 1,
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeOneConniveLandFromHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Island"
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
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mountain",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeOneConniveLandFromLib",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1,
          "tapped": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Convoke Fixture",
          "fixtureSource": true,
          "typeLine": "Artifact",
          "oracleText": "{0}: Tap target creature you control.\n{B}: Untap target creature you control."
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
          "source": "Convoke Fixture",
          "ability": "{0}: Tap target creature",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Convoke Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLibraryCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Swamp",
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 4
        },
        {
          "op": "saveRuntimeCheckpoint",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertLibraryCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Convoke Fixture",
          "ability": "Untap target creature",
          "target": "Grizzly Bears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Convoke Fixture",
          "abilityKind": "Activated",
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true,
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 3
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Swamp",
          "tapped": true,
          "count": 4,
          "seatIndependent": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 1,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 0
        },
        {
          "op": "assertTappedCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true,
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Mountain"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertLibraryCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mountain",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Island",
          "count": 0,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1,
          "seatIndependent": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": null,
          "count": 2,
          "seatIndependent": true
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2,
          "seatIndependent": true
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeOneConniveNonLandFromHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Felhide Minotaur",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Gray Ogre"
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
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeOneConniveNonLandFromLib",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Felhide Minotaur",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Felhide Minotaur"
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
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 3,
          "toughness": 3
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeTwoConniveMixed",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            },
            {
              "source": "Felhide Minotaur",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 2
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Felhide Minotaur",
          "tapped": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "select_objects",
          "description": "Choose a permanent to connive next",
          "options": [
            "Grizzly Bears",
            "Felhide Minotaur"
          ],
          "min": 1,
          "max": 1
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose a permanent to connive next",
          "choices": [
            "Grizzly Bears"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose 1 card to discard",
          "choices": [
            "Gray Ogre"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose 1 card to discard",
          "choices": [
            "Island"
          ]
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
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Felhide Minotaur",
          "power": 2,
          "toughness": 3
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeTwoConniveMixedOtherOrder",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            },
            {
              "source": "Felhide Minotaur",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 2
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Felhide Minotaur",
          "tapped": true
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "select_objects",
          "description": "Choose a permanent to connive next",
          "options": [
            "Felhide Minotaur",
            "Grizzly Bears"
          ],
          "min": 1,
          "max": 1
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose a permanent to connive next",
          "choices": [
            "Felhide Minotaur"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose 1 card to discard",
          "choices": [
            "Gray Ogre"
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose 1 card to discard",
          "choices": [
            "Island"
          ]
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
          "player": 1,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Island",
          "count": 0
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Felhide Minotaur",
          "power": 3,
          "toughness": 4
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeTwoWithControlChange",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Act of Aggression",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Felhide Minotaur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            },
            {
              "source": "Felhide Minotaur",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 2
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Felhide Minotaur",
          "tapped": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Act of Aggression",
          "target": {
            "groups": [
              [
                "Grizzly Bears"
              ]
            ]
          }
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "hybrid_payment",
          "choices": [
            "2 life (Phyrexian)"
          ]
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "hybrid_payment",
          "choices": [
            "2 life (Phyrexian)"
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Act of Aggression",
          "count": 1,
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Felhide Minotaur"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose 1 card to discard",
          "choices": [
            "Doom Blade"
          ]
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Grizzly Bears"
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "Choose 1 card to discard",
          "choices": [
            "Island"
          ]
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Grizzly Bears",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Felhide Minotaur",
          "power": 3,
          "toughness": 4
        }
      ]
    },
    {
      "name": "LethalSchemeConvokeOneThatGetsKilled",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lethal Scheme",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "clearZone",
          "player": 0,
          "zone": "library"
        },
        {
          "op": "clearZone",
          "player": 1,
          "zone": "library"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Gray Ogre",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Lethal Scheme",
          "target": "Elite Vanguard"
        },
        {
          "op": "setManaPayment",
          "player": 0,
          "source": "Lethal Scheme",
          "alternatives": [
            {
              "source": "Grizzly Bears",
              "kind": "convoke"
            }
          ]
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Lethal Scheme",
          "objectTarget": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "colors": "B",
          "amount": 1
        },
        {
          "op": "assertTapped",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears",
          "tapped": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Doom Blade",
          "target": {
            "groups": [
              [
                "Grizzly Bears"
              ]
            ]
          }
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Doom Blade",
          "count": 1,
          "objectTarget": "Grizzly Bears"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Grizzly Bears"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose 1 card to discard",
          "choices": [
            "Gray Ogre"
          ]
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
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
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Gray Ogre",
          "count": 1
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
