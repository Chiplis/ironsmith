import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/TaleOfTinuvielTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tale of Tinuviel",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Tale of Tinuviel"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "POSTCOMBAT_MAIN",
          "ability": 0,
          "expected": "Grizzly Bears"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertAbility",
          "player": "before III, Grizzly Bears indestructible",
          "name": 5,
          "ability": "UPKEEP",
          "expected": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "assertAbility",
          "player": "after III, Grizzly Bears not indestructible",
          "name": 5,
          "ability": "POSTCOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "assertAbility",
          "player": "after III, Grizzly Bears lifelink",
          "name": 5,
          "ability": "POSTCOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "assertAbility",
          "player": "after III, Memnite lifelink",
          "name": 5,
          "ability": "POSTCOMBAT_MAIN",
          "expected": 0
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 6,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "Lifelink",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Memnite",
          "ability": "Lifelink",
          "expected": false
        }
      ]
    }
  ]
});
