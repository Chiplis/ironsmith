import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/damage/DealsCombatDamageTriggerTest.java",
  "tests": [
    {
      "name": "triggerSourceDealsDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Drinker of Sorrow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Drinker of Sorrow",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Drinker of Sorrow",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "noTriggerOtherDealsDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Drinker of Sorrow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Memnite",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Drinker of Sorrow",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        }
      ]
    },
    {
      "name": "triggerTwoSourcesDealDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Drinker of Sorrow",
          "count": 2
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Drinker of Sorrow",
          "defender": 1
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Drinker of Sorrow",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whenever"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Drinker of Sorrow"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Drinker of Sorrow"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 10
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Drinker of Sorrow",
          "count": 2
        }
      ]
    }
  ]
});
