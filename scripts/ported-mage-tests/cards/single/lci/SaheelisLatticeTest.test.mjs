import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lci/SaheelisLatticeTest.java",
  "tests": [
    {
      "name": "testSaheelisLattice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Saheeli's Lattice",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Balamb T-Rexaur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Zetalpa, Primal Dawn",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Craft with one"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Balamb T-Rexaur^Zetalpa, Primal Dawn"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Mastercraft Raptor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mastercraft Raptor",
          "power": 10,
          "toughness": 4
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Balamb T-Rexaur",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Zetalpa, Primal Dawn",
          "count": 1
        }
      ]
    }
  ]
});
