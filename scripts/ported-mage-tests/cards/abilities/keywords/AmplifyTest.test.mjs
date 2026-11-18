import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/abilities/keywords/AmplifyTest.java",
  "tests": [
    {
      "name": "test_Amplify_OneCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kilnmouth Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kilnmouth Dragon"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "power": 8,
          "toughness": 8
        }
      ]
    },
    {
      "name": "test_Amplify_TwoCards",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Phantasmal Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Kilnmouth Dragon"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Kilnmouth Dragon^Phantasmal Dragon"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "power": 11,
          "toughness": 11
        }
      ]
    },
    {
      "name": "test_Amplify_WithClone",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kilnmouth Dragon@original",
          "count": 1
        },
        {
          "op": "addCard",
          "player": 0,
          "zone": "HAND",
          "name": "Kilnmouth Dragon@reveal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Clone",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Phantasmal Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 4
        },
        {
          "op": "castSpell",
          "name": "Kilnmouth Dragon",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@original"
        },
        {
          "op": "assertStackObject",
          "source": "Kilnmouth Dragon",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setObjectChoices",
          "player": 0,
          "description": "Choose any number of cards from your hand that share a creature type with this creature to reveal for amplify",
          "choices": [
            "Kilnmouth Dragon"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "@original",
          "counter": "+1/+1",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "@original",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "@reveal",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Clone",
          "player": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Clone",
          "count": 1,
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "description": "Choose what to copy as this permanent enters",
          "kind": "options",
          "choices": [
            {
              "description": "Enter as a copy of Kilnmouth Dragon",
              "object": "@original"
            }
          ]
        },
        {
          "op": "setObjectChoices",
          "player": 1,
          "description": "Choose any number of cards from your hand that share a creature type with this creature to reveal for amplify",
          "choices": [
            "Phantasmal Dragon"
          ]
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kilnmouth Dragon",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Kilnmouth Dragon",
          "counter": "+1/+1",
          "count": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Kilnmouth Dragon",
          "power": 8,
          "toughness": 8
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "name": "Phantasmal Dragon",
          "count": 1
        }
      ]
    },
    {
      "name": "test_Amplify_PutOntoBattlefieldFromHand",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Through the Breach",
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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Through the Breach"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Kilnmouth Dragon"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kilnmouth Dragon",
          "power": 5,
          "toughness": 5
        }
      ]
    }
  ]
});
