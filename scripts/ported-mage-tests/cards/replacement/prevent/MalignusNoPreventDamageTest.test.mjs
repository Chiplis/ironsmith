import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/replacement/prevent/MalignusNoPreventDamageTest.java",
  "tests": [
    {
      "name": "testBlockByCreatureWithProRed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vedalken Outlander",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Malignus",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
          "target": 0
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Malignus",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Vedalken Outlander",
          "attacker": "Malignus"
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
          "op": "assertLife",
          "player": 0,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Malignus",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Malignus",
          "power": 9,
          "toughness": 9
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vedalken Outlander",
          "count": 0
        }
      ]
    },
    {
      "name": "testBlockAnotherWithProRed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vedalken Outlander",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ogre Resister",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Ogre Resister",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Vedalken Outlander",
          "attacker": "Ogre Resister"
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ogre Resister",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vedalken Outlander",
          "count": 1
        }
      ]
    }
  ]
});
