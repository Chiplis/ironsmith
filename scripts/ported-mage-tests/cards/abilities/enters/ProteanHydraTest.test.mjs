import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/enters/ProteanHydraTest.java",
  "tests": [
    {
      "name": "testEnteringWithCounters",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Protean Hydra",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Protean Hydra",
          "holdStack": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=4"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Protean Hydra",
          "count": 1,
          "isSpell": true,
          "xValue": 4
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setStrictChooseMode",
          "value": false
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
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertTappedCount",
          "player": 0,
          "name": "Forest",
          "tapped": true,
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Protean Hydra",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Protean Hydra",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Protean Hydra",
          "counter": "+1/+1",
          "count": 4
        }
      ]
    }
  ]
});
