import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh3/SuppressionRayTest.java",
  "tests": [
    {
      "name": "test_Simple",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Zodiac Dog",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Zodiac Goat",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Zodiac Horse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Zodiac Rabbit",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Zodiac Pig",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Suppression Ray",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aethertide Whale",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 11
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aethertide Whale"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Aethertide Whale",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Aethertide Whale",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "name": 0,
          "count": 6,
          "player": 0,
          "counter": "ENERGY",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Suppression Ray",
          "target": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Suppression Ray",
          "count": 1
        },
        {
          "op": "setOptionChoices",
          "player": 0,
          "kind": "boolean",
          "description": "Pay any amount of {E}",
          "choices": [
            "Yes"
          ]
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "number",
          "description": "Choose how much {E} to pay",
          "value": 3
        },
        {
          "op": "assertChoiceMenu",
          "player": 0,
          "kind": "select_objects",
          "description": "Choose up to 3 creatures",
          "options": [
            "Zodiac Goat",
            "Zodiac Horse",
            "Zodiac Rabbit",
            "Zodiac Pig"
          ]
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Zodiac Pig^Zodiac Rabbit"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
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
          "op": "assertTapped",
          "name": "Zodiac Dog",
          "tapped": false,
          "player": 0
        },
        {
          "op": "assertTapped",
          "name": "Zodiac Goat",
          "tapped": true,
          "player": 1
        },
        {
          "op": "assertTapped",
          "name": "Zodiac Horse",
          "tapped": true,
          "player": 1
        },
        {
          "op": "assertTapped",
          "name": "Zodiac Rabbit",
          "tapped": true,
          "player": 1
        },
        {
          "op": "assertTapped",
          "name": "Zodiac Pig",
          "tapped": true,
          "player": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Zodiac Dog",
          "counter": "STUN",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Zodiac Goat",
          "counter": "STUN",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Zodiac Horse",
          "counter": "STUN",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Zodiac Rabbit",
          "counter": "STUN",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 1,
          "name": "Zodiac Pig",
          "counter": "STUN",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": 0,
          "counter": "ENERGY",
          "count": 3
        }
      ]
    }
  ]
});
