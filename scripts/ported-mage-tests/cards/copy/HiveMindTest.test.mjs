import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/HiveMindTest.java",
  "tests": [
    {
      "name": "testTransform",
      "operations": [
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
          "player": 0,
          "name": "Hive Mind",
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
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": 0
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Lightning Bolt",
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
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 17
        }
      ]
    },
    {
      "name": "testChaliceOfTtheVoid",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hive Mind",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pact of the Titan",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Chalice of the Void",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pact of the Titan"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Pact of the Titan",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Giant Token",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Giant Token",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertWonTheGame(playerA)"
        },
        {
          "op": "unsupported",
          "source": "assertLostTheGame(playerB)"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
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
