import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/SorinLordOfInnistradTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin, Lord of Innistrad",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Create a 1/1 black Vampire creature token with lifelink."
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sorin, Lord of Innistrad",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vampire Token",
          "count": 1
        }
      ]
    },
    {
      "name": "testCard2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin, Lord of Innistrad",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.UPKEEP, playerA, \"Sorin, Lord of Innistrad\", CounterType.LOYALTY, 1)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-2: You get an emblem with "
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-2: You get an emblem with "
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sorin, Lord of Innistrad",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sejiri Merfolk",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Sejiri Merfolk",
          "power": 4,
          "toughness": 1
        }
      ]
    },
    {
      "name": "testCard3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sorin, Lord of Innistrad",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Angel of Mercy",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "addCounters(1, PhaseStep.UPKEEP, playerA, \"Sorin, Lord of Innistrad\", CounterType.LOYALTY, 3)"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-6: ",
          "target": "Craw Wurm^Angel of Mercy"
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
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Mercy",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Sorin, Lord of Innistrad",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Craw Wurm",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Angel of Mercy",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Angel of Mercy",
          "count": 1
        }
      ]
    }
  ]
});
