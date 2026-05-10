import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/DevourTest.java",
  "tests": [
    {
      "name": "Wurm_NoDevour",
      "operations": []
    },
    {
      "name": "Wurm_OneDevour",
      "operations": []
    },
    {
      "name": "Wurm_TwoDevour",
      "operations": []
    },
    {
      "name": "Wurm_ThreeDevour",
      "operations": []
    },
    {
      "name": "Wurm_IllegalDevour",
      "operations": []
    },
    {
      "name": "Thromok_NoDevour",
      "operations": []
    },
    {
      "name": "Thromok_OneDevour",
      "operations": []
    },
    {
      "name": "Thromok_TwoDevour",
      "operations": []
    },
    {
      "name": "Thromok_ThreeDevour",
      "operations": []
    },
    {
      "name": "Thromok_IllegalDevour",
      "operations": []
    },
    {
      "name": "Hobbit_NoDevour",
      "operations": []
    },
    {
      "name": "Hobbit_OneDevour",
      "operations": []
    },
    {
      "name": "Hobbit_IllegalDevour",
      "operations": []
    },
    {
      "name": "Caprichrome_NoDevour",
      "operations": []
    },
    {
      "name": "Caprichrome_OneDevour",
      "operations": []
    },
    {
      "name": "Caprichrome_TwoDevour",
      "operations": []
    },
    {
      "name": "Caprichrome_ThreeDevour",
      "operations": []
    },
    {
      "name": "Caprichrome_IllegalDevour",
      "operations": []
    },
    {
      "name": "Hatchling_NoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
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
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": false
        }
      ]
    },
    {
      "name": "Hatchling_OneDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Enatu Golem"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "Hatchling_TwoDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Enatu Golem^Grizzly Bears"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "Hatchling_ThreeDevour",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Chromatic Orrery",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Hellkite Hatchling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enatu Golem",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Darksteel Relic",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Angrath, Captain of Chaos",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Hellkite Hatchling"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Enatu Golem^Grizzly Bears^Silvercoat Lion"
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
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Hellkite Hatchling",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "Hatchling_IllegalDevour",
      "operations": []
    },
    {
      "name": "Chomper_NoDevour",
      "operations": []
    },
    {
      "name": "Chomper_OneDevour",
      "operations": []
    },
    {
      "name": "Chomper_TwoDevour",
      "operations": []
    },
    {
      "name": "Chomper_ThreeDevour",
      "operations": []
    },
    {
      "name": "Chomper_IllegalDevour",
      "operations": []
    },
    {
      "name": "hellionNoDevour",
      "operations": []
    },
    {
      "name": "hellionOneDevour",
      "operations": []
    },
    {
      "name": "hellionTwoDevour",
      "operations": []
    },
    {
      "name": "hellionThreeDevour",
      "operations": []
    },
    {
      "name": "hellionIllegalDevour",
      "operations": []
    }
  ]
});
