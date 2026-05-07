import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ulg/GoblinWelderTest.java",
  "tests": [
    {
      "name": "testSacrificeDiesTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Goblin Welder",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Blood Aspirant",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}:"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Wurmcoil Engine"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Darksteel Relic"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null,
          "once": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever you sacrifice a permanent"
        },
        {
          "op": "assertStackSize",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "count": 2
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
          "name": "Wurmcoil Engine",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Blood Aspirant",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Phyrexian Wurm Token",
          "count": 2
        }
      ]
    }
  ]
});
