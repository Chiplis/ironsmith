import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/thb/TheTriumphOfAnaxTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "The Triumph of Anax",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "The Triumph of Anax"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "power": 0,
          "toughness": "Memnite"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 0,
          "expected": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertPowerToughness",
          "player": "T3: Memnite is 3/1",
          "name": 3,
          "power": "POSTCOMBAT_MAIN",
          "toughness": 0
        },
        {
          "op": "assertAbility",
          "player": "T3: Memnite has trample",
          "name": 3,
          "ability": "POSTCOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertPowerToughness",
          "player": "T4: Memnite is 4/1",
          "name": 5,
          "power": "POSTCOMBAT_MAIN",
          "toughness": 0
        },
        {
          "op": "assertAbility",
          "player": "T4: Memnite has trample",
          "name": 5,
          "ability": "POSTCOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "The Triumph of Anax",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerB, \"Ornithopter\", 1)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Memnite",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Memnite",
          "ability": "Trample",
          "expected": false
        }
      ]
    }
  ]
});
