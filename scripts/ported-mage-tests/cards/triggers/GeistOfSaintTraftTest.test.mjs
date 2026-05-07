import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/GeistOfSaintTraftTest.java",
  "tests": [
    {
      "name": "testTokenwillBeCreated",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Geist of Saint Traft",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Geist of Saint Traft",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "COMBAT_DAMAGE"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Geist of Saint Traft",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Geist of Saint Traft",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Angel Token",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Angel Token",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    },
    {
      "name": "testTokenwillBeExiled",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Geist of Saint Traft",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Geist of Saint Traft",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Geist of Saint Traft",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Geist of Saint Traft",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Angel Token",
          "count": 0
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 14
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
