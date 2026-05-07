import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/EvokeTest.java",
  "tests": [
    {
      "name": "testCreatureComesIntoPlay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shriekmaw",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Exhume",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shriekmaw"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Evoke alternative cost: {1}{B} (source: Shriekmaw"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When this permanent enters, if its evoke cost was paid, its controller sacrifices it."
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Exhume"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Silvercoat Lion"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Shriekmaw"
        },
        {
          "op": "addTarget",
          "player": 0,
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Exhume",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shriekmaw",
          "count": 1
        }
      ]
    },
    {
      "name": "testControllerSacrifices",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Wrong Turn",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mulldrifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Proper Burial",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Mulldrifter"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast with Evoke alternative cost: {2}{U} (source: Mulldrifter"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, draw"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Wrong Turn"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mulldrifter"
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
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mulldrifter",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Wrong Turn",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 22
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        }
      ]
    }
  ]
});
