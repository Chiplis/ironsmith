import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/ElbrusTheBindingBladeTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elbrus, the Binding Blade",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Equip {1}",
          "target": "Air Elemental"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Air Elemental",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
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
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Air Elemental",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elbrus, the Binding Blade",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Withengar Unbound",
          "count": 1
        }
      ]
    }
  ]
});
