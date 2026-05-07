import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bfz/HalimarTidecallerTest.java",
  "tests": [
    {
      "name": "testMe",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Halimar Tidecaller",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Treetop Village",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{G}:"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Treetop Village",
          "defender": 1
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
          "life": 17
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Treetop Village",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Treetop Village",
          "ability": "Trample",
          "expected": true
        }
      ]
    }
  ]
});
