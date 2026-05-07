import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/tmp/PhyrexianSplicerTest.java",
  "tests": [
    {
      "name": "test_Normal",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Phyrexian Splicer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma, Angel of Wrath",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Augur il-Vec",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Akroma, Angel of Wrath"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Augur il-Vec"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}, Choose"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Akroma, Angel of Wrath"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Augur il-Vec"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Trample"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Akroma, Angel of Wrath"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "PRECOMBAT_MAIN",
          "ability": 0,
          "expected": "Augur il-Vec"
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
        }
      ]
    }
  ]
});
