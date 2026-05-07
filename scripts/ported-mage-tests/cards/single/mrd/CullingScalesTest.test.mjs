import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mrd/CullingScalesTest.java",
  "tests": [
    {
      "name": "testCullingScalesBasic",
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
          "name": "Culling Scales",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Siege Rhino",
          "count": 1
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
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Culling Scales",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Siege Rhino",
          "count": 1
        }
      ]
    },
    {
      "name": "testCullingScalesPlusHexproof",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bassara Tower Archer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Culling Scales",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Siege Rhino",
          "count": 1
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
          "player": 1,
          "name": "Bassara Tower Archer",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Culling Scales",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Siege Rhino",
          "count": 1
        }
      ]
    },
    {
      "name": "testCullingScalesFizzleByMakingLowerCostedPermanent",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Raise the Alarm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elvish Visionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Culling Scales",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elvish Visionary"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Raise the Alarm",
          "target": "At the beginning of"
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
          "player": 1,
          "name": "Soldier Token",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Elvish Visionary",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Plains",
          "count": 5
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Culling Scales",
          "count": 1
        }
      ]
    }
  ]
});
