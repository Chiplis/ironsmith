import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/protection/EightAndAHalfTailsTest.java",
  "tests": [
    {
      "name": "testProtectingPlaneswalker",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Karn, the Great Creator",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Eight-and-a-Half-Tails",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Angel of Destiny",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Angel of Destiny",
          "defender": "Karn, the Great Creator"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "ability": "{1}{W}: Target permanent you control gains protection from white until end of turn."
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Karn, the Great Creator"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Karn, the Great Creator",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Karn, the Great Creator",
          "counter": "LOYALTY",
          "count": 5
        }
      ]
    }
  ]
});
