import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/soi/EssenceFluxTest.java",
  "tests": [
    {
      "name": "test_ExileSpiritNonSpirit",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 2
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
          "player": 0,
          "name": "Abuelo, Ancestral Echo",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Essence Flux",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Essence Flux"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Essence Flux"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Abuelo, Ancestral Echo"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Memnite",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Abuelo, Ancestral Echo",
          "counter": "P1P1",
          "count": 1
        }
      ]
    }
  ]
});
