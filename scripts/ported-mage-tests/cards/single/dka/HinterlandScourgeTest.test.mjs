import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/HinterlandScourgeTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hinterland Hermit",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "BEGIN_COMBAT"
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
          "name": "Hinterland Hermit",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hinterland Scourge",
          "count": 1
        }
      ]
    },
    {
      "name": "testCard1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hinterland Hermit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Hinterland Scourge",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
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
          "name": "Hinterland Hermit",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hinterland Scourge",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 0
        }
      ]
    },
    {
      "name": "testCard2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hinterland Hermit",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Hinterland Scourge",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_COMBAT"
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
          "life": 17
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hinterland Hermit",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Hinterland Scourge",
          "count": 1
        }
      ]
    }
  ]
});
