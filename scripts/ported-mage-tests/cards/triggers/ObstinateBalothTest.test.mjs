import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/ObstinateBalothTest.java",
  "tests": [
    {
      "name": "TestLifeGainIfForcedToDiscard",
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
          "name": "Raven's Crime",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Obstinate Baloth",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Raven's Crime",
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Raven's Crime",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Obstinate Baloth",
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
          "life": 24
        }
      ]
    },
    {
      "name": "TestWithSmallpox",
      "operations": [
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
          "player": 0,
          "name": "Smallpox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Obstinate Baloth",
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
          "name": "Smallpox"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Obstinate Baloth"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Obstinate Baloth"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Swamp"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Smallpox",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Obstinate Baloth",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Obstinate Baloth",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 19
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 23
        }
      ]
    }
  ]
});
