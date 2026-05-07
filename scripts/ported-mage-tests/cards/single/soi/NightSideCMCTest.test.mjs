import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/soi/NightSideCMCTest.java",
  "tests": [
    {
      "name": "insectileAbberationRepealXis1Test",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Repeal",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Repeal"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=1"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Repeal",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insectile Aberration",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 1
        }
      ]
    },
    {
      "name": "insectileAbberationEngeeredExplosivesSunburstIs1Test",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Lightning Bolt",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Engineered Explosives",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Engineered Explosives"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "X=1"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "BEGIN_COMBAT",
          "player": 1,
          "ability": "{2}"
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
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Engineered Explosives",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Insectile Aberration",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Delver of Secrets",
          "count": 1
        }
      ]
    }
  ]
});
