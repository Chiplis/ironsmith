import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/afr/Plus2MaceTest.java",
  "tests": [
    {
      "name": "test_NameProcessingByRegExp",
      "operations": [
        {
          "op": "addCard",
          "player": 0,
          "zone": "HAND",
          "name": "+2 Mace"
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "BATTLEFIELD",
          "name": "Plains",
          "count": 2
        },
        {
          "op": "assertCastMethods",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "+2 Mace",
          "methods": ["normal"],
          "methodLabel": "+2 Mace"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "+2 Mace"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "+2 Mace",
          "count": 1
        }
      ]
    }
  ]
});
