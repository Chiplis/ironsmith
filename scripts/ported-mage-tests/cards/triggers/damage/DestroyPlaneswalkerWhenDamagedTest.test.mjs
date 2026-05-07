import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/DestroyPlaneswalkerWhenDamagedTest.java",
  "tests": [
    {
      "name": "nullFilterTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "nullFilterDPwD",
          "player": 0,
          "name": "new DestroyPlaneswalkerWhenDamagedTriggeredAbility()",
          "custom": true,
          "oracleText": "None"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Phyrexian Walker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Chandra, Acolyte of Flame",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "nullFilterDPwD",
          "defender": "Chandra, Acolyte of Flame"
        },
        {
          "op": "block",
          "turn": 1,
          "player": 1,
          "blocker": "Phyrexian Walker",
          "attacker": "nullFilterDPwD"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "nullFilterDPwD",
          "defender": "Chandra, Acolyte of Flame"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "nullFilterDPwD",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Phyrexian Walker",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Chandra, Acolyte of Flame",
          "count": 1
        }
      ]
    }
  ]
});
