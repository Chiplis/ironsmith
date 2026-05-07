import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/restriction/CanBlockOnlyFlyingTest.java",
  "tests": [
    {
      "name": "testCannotBlockCreatureWithoutFlying",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vaporkin",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Vaporkin",
          "attacker": "Silvercoat Lion"
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Vaporkin",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        }
      ]
    },
    {
      "name": "testCanBlockCreatureWithFlying",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wingsteed Rider",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vaporkin",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Wingsteed Rider",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Vaporkin",
          "attacker": "Wingsteed Rider"
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
          "name": "Wingsteed Rider",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Vaporkin",
          "count": 0
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
        }
      ]
    },
    {
      "name": "testCantBlockFlyerAfterLosingFlying",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Archetype of Imagination",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vaporkin",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Archetype of Imagination",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Vaporkin",
          "attacker": "Archetype of Imagination"
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
          "name": "Archetype of Imagination",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Vaporkin",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 17
        }
      ]
    },
    {
      "name": "testCanBlockCreatureWhichGainedFlying",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Jump",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vaporkin",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Jump",
          "target": "Silvercoat Lion"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Silvercoat Lion",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 3,
          "player": 1,
          "blocker": "Vaporkin",
          "attacker": "Silvercoat Lion"
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
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Vaporkin",
          "count": 0
        }
      ]
    }
  ]
});
