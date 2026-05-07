import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dmc/CadricSoulKindlerTest.java",
  "tests": [
    {
      "name": "testOneCopy",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cadric, Soul Kindler",
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
          "name": "Isamaru, Hound of Konda",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isamaru, Hound of Konda"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "name": "Isamaru, Hound of Konda",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Isamaru, Hound of Konda",
          "count": 0
        }
      ]
    },
    {
      "name": "testTwoCopies",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Cadric, Soul Kindler",
          "count": 1
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
          "name": "Isamaru, Hound of Konda",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Isamaru, Hound of Konda"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Isamaru, Hound of Konda"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Isamaru, Hound of Konda"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
          "name": "Isamaru, Hound of Konda",
          "count": 3
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Isamaru, Hound of Konda",
          "count": 1
        }
      ]
    }
  ]
});
