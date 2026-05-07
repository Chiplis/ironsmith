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
