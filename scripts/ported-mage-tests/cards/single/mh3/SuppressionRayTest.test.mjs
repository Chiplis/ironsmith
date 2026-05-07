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
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Suppression Ray",
          "target": 1
        },
        {
          "op": "unsupported",
          "source": "setChoiceAmount(playerA, 3)"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Zodiac Pig^Zodiac Rabbit"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "TestPlayer.CHOICE_SKIP"
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
          "source": "assertTapped(\"Zodiac Dog\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Zodiac Goat\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Zodiac Horse\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Zodiac Rabbit\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Zodiac Pig\", true)"
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
