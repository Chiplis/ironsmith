import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c18/VarchildBetrayerOfKjeldorTest.java",
  "tests": [
    {
      "name": "testOpponentGetsSurvivorTokens",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Varchild, Betrayer of Kjeldor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Varchild, Betrayer of Kjeldor"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Varchild, Betrayer of Kjeldor",
          "defender": 1
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Varchild, Betrayer of Kjeldor",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Survivor Token",
          "count": 3
        }
      ]
    },
    {
      "name": "testGetControlEffect",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Varchild, Betrayer of Kjeldor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Irregular Cohort",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Swords to Plowshares",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Varchild, Betrayer of Kjeldor"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Varchild, Betrayer of Kjeldor",
          "defender": 1
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Swords to Plowshares",
          "target": "Varchild, Betrayer of Kjeldor"
        },
        {
          "op": "castSpell",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Irregular Cohort"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Swords to Plowshares",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Varchild, Betrayer of Kjeldor",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Survivor Token",
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Irregular Cohort",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Shapeshifter Token",
          "count": 1
        }
      ]
    }
  ]
});
