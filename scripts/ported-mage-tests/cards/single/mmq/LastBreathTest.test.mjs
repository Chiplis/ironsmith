import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mmq/LastBreathTest.java",
  "tests": [
    {
      "name": "testControllerGains",
      "operations": [
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
          "name": "Last Breath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wind Drake",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Last Breath",
          "target": "Wind Drake"
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
          "name": "Last Breath",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Wind Drake",
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
      "name": "testControllerChanged",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Last Breath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Wind Drake",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plateau",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Act of Treason",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Act of Treason",
          "target": "Wind Drake"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Last Breath",
          "target": "Wind Drake"
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
          "name": "Last Breath",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Wind Drake",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
