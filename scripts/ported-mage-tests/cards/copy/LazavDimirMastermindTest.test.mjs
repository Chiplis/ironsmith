import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/LazavDimirMastermindTest.java",
  "tests": [
    {
      "name": "testCopySimpleCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lazav, Dimir Mastermind@lazav",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Assault Griffin",
          "count": 5
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
          "target": 1
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
          "name": "Lazav, Dimir Mastermind",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lazav"
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
          "name": "Lazav, Dimir Mastermind",
          "ability": "Flying",
          "expected": true
        }
      ]
    },
    {
      "name": "testRatsHaveDeathtouch",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lazav, Dimir Mastermind@lazav",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gutter Skulk",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Ogre Slumlord",
          "count": 5
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
          "target": 1
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
          "name": "Lazav, Dimir Mastermind",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lazav"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Ogre",
              "expected": true
            },
            {
              "property": "subtype",
              "value": "Rogue",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gutter Skulk",
          "ability": "Deathtouch",
          "expected": true
        }
      ]
    },
    {
      "name": "testCopyNightveilSpecter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lazav, Dimir Mastermind@lazav",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Nightveil Specter",
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
          "target": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Lazav, Dimir Mastermind",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "power": 2,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lazav"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Specter",
              "expected": true
            },
            {
              "property": "supertype",
              "value": "Legendary",
              "expected": true
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "ability": "Flying",
          "expected": true
        }
      ]
    },
    {
      "name": "testCopyMultipleTimes",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lazav, Dimir Mastermind@lazav",
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
          "zone": "LIBRARY",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Nightveil Specter",
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
          "target": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Target player mills a card.",
          "target": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lazav"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Cat",
              "expected": true
            },
            {
              "property": "supertype",
              "value": "Legendary",
              "expected": true
            }
          ]
        }
      ]
    },
    {
      "name": "testCopyAfterReanimation",
      "operations": [
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
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lazav, Dimir Mastermind@lazav",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Assault Griffin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Tribute to Hunger",
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
          "target": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Tribute to Hunger"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Reanimate",
          "target": "Lazav, Dimir Mastermind"
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
          "player": 1,
          "name": "Tribute to Hunger",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Reanimate",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 16
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lazav"
          },
          "count": 1,
          "checks": [
            {
              "property": "subtype",
              "value": "Griffin",
              "expected": false
            }
          ]
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "ability": "Flying",
          "expected": false
        }
      ]
    },
    {
      "name": "testCopyCreatureExiled",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lazav, Dimir Mastermind@lazav",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Rest in Peace",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 1,
          "name": "Assault Griffin",
          "count": 5
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
          "target": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Rest in Peace"
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
          "name": "Lazav, Dimir Mastermind",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Lazav, Dimir Mastermind",
          "power": 3,
          "toughness": 2
        },
        {
          "op": "assertEachObject",
          "selector": {
            "zone": "battlefield",
            "player": 0,
            "alias": "@lazav"
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
          "name": "Lazav, Dimir Mastermind",
          "ability": "Flying",
          "expected": true
        }
      ]
    }
  ]
});
