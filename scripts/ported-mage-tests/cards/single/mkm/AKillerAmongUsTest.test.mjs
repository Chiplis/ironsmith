import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mkm/AKillerAmongUsTest.java",
  "tests": [
    {
      "name": "test_TargetChosenCreatureType",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "A Killer Among Us",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "A Killer Among Us"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Merfolk"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Human Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Merfolk Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Token",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Merfolk Token",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Merfolk Token"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "A Killer Among Us",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Merfolk Token",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Merfolk Token",
          "ability": "Deathtouch",
          "expected": true
        }
      ]
    },
    {
      "name": "test_TargetNotChosenCreatureType",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "A Killer Among Us",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "A Killer Among Us"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Merfolk"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Human Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Merfolk Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Goblin Token",
          "count": 1
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Human Token",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Human Token"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Human Token",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Human Token",
          "ability": "Deathtouch",
          "expected": false
        }
      ]
    },
    {
      "name": "test_CopyTrigger_TargetLastChosenCreatureType",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taiga",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lithoform Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "A Killer Among Us",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tremor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "A Killer Among Us"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}",
          "target": "stack ability (When"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Human"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Merfolk"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Merfolk Token",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Merfolk Token"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Tremor"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertCounterCount",
          "player": 0,
          "name": "Merfolk Token",
          "counter": "P1P1",
          "count": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Merfolk Token",
          "ability": "Deathtouch",
          "expected": true
        }
      ]
    },
    {
      "name": "test_CopyTrigger_TargetNotLastChosenCreatureType",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Taiga",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lithoform Engine",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "A Killer Among Us",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Tremor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "A Killer Among Us"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, {T}",
          "target": "stack ability (When"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Human"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Merfolk"
        },
        {
          "op": "attack",
          "turn": 3,
          "player": 0,
          "attacker": "Human Token",
          "defender": 1
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "DECLARE_BLOCKERS",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Human Token"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Tremor"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "name": "Human Token",
          "count": 0
        }
      ]
    }
  ]
});
