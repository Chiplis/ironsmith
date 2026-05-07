import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/eve/PyrrhicRevivalTest.java",
  "tests": [
    {
      "name": "test_PyrrhicRevival",
      "operations": [
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Cathedral Sanctifier",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Cathedral Sanctifier",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pyrrhic Revival",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pyrrhic Revival"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, you gain 3 life."
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
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Cathedral Sanctifier",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cathedral Sanctifier",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Cathedral Sanctifier",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 26
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
