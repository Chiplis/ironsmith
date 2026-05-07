import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/rules/TriggerAbilityOnlyLimitedTimesTest.java",
  "tests": [
    {
      "name": "testTriggerOnceEachTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enduring Innocence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 1
        }
      ]
    },
    {
      "name": "testTriggerTwiceSameTurnIfBlinked",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Momentary Blink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Enduring Innocence",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Momentary Blink",
          "target": "Enduring Innocence"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Enduring Innocence",
          "count": 1
        }
      ]
    },
    {
      "name": "testTriggerOnceEachGame",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Acrobatic Cheerleader",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Acrobatic Cheerleader",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Acrobatic Cheerleader",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Acrobatic Cheerleader",
          "counter": "FLYING",
          "count": 1
        }
      ]
    },
    {
      "name": "testTriggerTwiceSameGameIfBlinked",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Acrobatic Cheerleader",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Momentary Blink",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Acrobatic Cheerleader",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "END_TURN",
          "player": 0,
          "name": "Momentary Blink",
          "target": "Acrobatic Cheerleader"
        },
        {
          "op": "attack",
          "turn": 5,
          "player": 0,
          "attacker": "Acrobatic Cheerleader",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Acrobatic Cheerleader",
          "counter": "FLYING",
          "count": 1
        }
      ]
    }
  ]
});
