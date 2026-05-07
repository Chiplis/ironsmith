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
          "name": "Volrath's Shapeshifter",
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
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mind Control",
          "target": "Assault Griffin"
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
          "name": "Dutiful Thrull",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dutiful Thrull",
          "power": 1,
          "toughness": 1
        }
      ]
    }
  ]
});
