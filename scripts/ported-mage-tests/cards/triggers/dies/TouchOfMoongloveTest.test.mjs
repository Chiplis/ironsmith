import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/triggers/dies/TouchOfMoongloveTest.java",
  "tests": [
    {
      "name": "testDiesAndControllerDamage",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cleric of the Forward Order",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Guardian Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Swamp",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Touch of Moonglove",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Pharika's Disciple",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Cleric of the Forward Order"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Cleric of the Forward Order",
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
          "source": "Cleric of the Forward Order",
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
          "op": "attack",
          "turn": 2,
          "player": 1,
          "attacker": "Pharika's Disciple",
          "defender": 0,
          "expectedLegal": true
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Cleric of the Forward Order",
          "attacker": "Pharika's Disciple"
        },
        {
          "op": "block",
          "turn": 2,
          "player": 0,
          "blocker": "Guardian Automaton",
          "attacker": "Pharika's Disciple"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "player": 1,
          "name": "Touch of Moonglove",
          "target": "Pharika's Disciple"
        },
        {
          "op": "assertStackObject",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "source": "Touch of Moonglove",
          "count": 1,
          "objectTarget": "Pharika's Disciple"
        },
        {
          "op": "waitStackResolved",
          "turn": 2,
          "phase": "DECLARE_BLOCKERS",
          "resolutions": 1
        },
        {
          "op": "setOptionChoices",
          "player": 1,
          "kind": "distribution",
          "description": "Assign 3 combat damage from Pharika's Disciple (assign exactly 3 total)",
          "choices": [
            "Cleric of the Forward Order",
            "Guardian Automaton",
            "Guardian Automaton"
          ]
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Touch of Moonglove",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Cleric of the Forward Order",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Pharika's Disciple",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Guardian Automaton",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        }
      ]
    }
  ]
});
