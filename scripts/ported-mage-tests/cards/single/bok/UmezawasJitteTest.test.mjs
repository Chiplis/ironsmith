import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bok/UmezawasJitteTest.java",
  "tests": [
    {
      "name": "testTrampleSingleDamageTrigger",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Umezawa's Jitte",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Spiked Baloth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Darksteel Myr",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip",
          "target": "Spiked Baloth"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Spiked Baloth",
          "defender": 1
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Memnite",
          "attacker": "Spiked Baloth"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Darksteel Myr",
          "attacker": "Spiked Baloth"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=1"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Darksteel Myr",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Umezawa's Jitte",
          "counter": "CHARGE",
          "count": 2
        }
      ]
    }
  ]
});
