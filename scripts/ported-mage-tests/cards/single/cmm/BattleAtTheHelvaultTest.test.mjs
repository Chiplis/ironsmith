import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/cmm/BattleAtTheHelvaultTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Battle at the Helvault",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Battle at the Helvault"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Grizzly Bears"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Bear Cub",
          "count": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Augmenting Automaton"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bear Cub"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Avacyn",
          "count": 1
        }
      ]
    }
  ]
});
