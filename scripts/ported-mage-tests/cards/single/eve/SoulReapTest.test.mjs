import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/eve/SoulReapTest.java",
  "tests": [
    {
      "name": "testSpell",
      "operations": [
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
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soul Reap",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Muck Rats",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Muck Rats"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Soul Reap",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Muck Rats",
          "count": 1
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
          "name": "Soul Reap",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "testNoSpell",
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Soul Reap",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Soul Reap",
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 0
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
          "name": "Soul Reap",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    }
  ]
});
