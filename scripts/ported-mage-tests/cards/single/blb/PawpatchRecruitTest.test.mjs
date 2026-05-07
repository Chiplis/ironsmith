import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/blb/PawpatchRecruitTest.java",
  "tests": [
    {
      "name": "testCopiedTriggerAbility",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pawpatch Recruit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Panharmonicon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Chrome Prowler",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Chrome Prowler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pawpatch Recruit"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Whenever a creature"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Pawpatch Recruit"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Bear Cub"
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
          "op": "unsupported",
          "source": "assertTapped(paw, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(cub, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Pawpatch Recruit",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Bear Cub",
          "counter": "P1P1",
          "count": 1
        }
      ]
    }
  ]
});
