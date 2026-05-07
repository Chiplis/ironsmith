import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh2/PersistTest.java",
  "tests": [
    {
      "name": "test_Persist_Card",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cathedral Sanctifier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Persist",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Persist",
          "target": "Cathedral Sanctifier"
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
          "name": "Cathedral Sanctifier",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cathedral Sanctifier",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    }
  ]
});
