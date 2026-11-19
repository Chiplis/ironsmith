import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/VolrathsShapshifterTest.java",
  "tests": [
    {
      "name": "testCopySimpleCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Assault Griffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Assault Griffin",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Assault Griffin",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Assault Griffin"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Griffin",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Assault Griffin",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Assault Griffin",
          "kind": "activated",
          "textPattern": "\\bdiscard a card",
          "count": 1
        }
      ]
    },
    {
      "name": "testLosingCopy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Codex Shredder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Assault Griffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target player mills a card.",
          "target": 0
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "power": 0,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "name": "Volrath's Shapeshifter"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Shapeshifter",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "kind": "activated",
          "textPattern": "\\bdiscard a card",
          "count": 1
        }
      ]
    },
    {
      "name": "testControlChange",
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
          "name": "Mind Control",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Forest",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dutiful Thrull",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Volrath's Shapeshifter@shifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Assault Griffin",
          "count": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 1,
            "alias": "@shifter"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Assault Griffin",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Mind Control",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "@shifter"
        },
        {
          "op": "assertStackObject",
          "source": "Mind Control",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Volrath's Shapeshifter"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@shifter"
          },
          "count": 1,
          "checks": []
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dutiful Thrull",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@shifter",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@shifter"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Thrull",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbilityKindCount",
          "player": 0,
          "name": "@shifter",
          "kind": "activated",
          "textPattern": "\\{2\\}.*Discard a card",
          "count": 1
        }
      ]
    }
  ]
});
