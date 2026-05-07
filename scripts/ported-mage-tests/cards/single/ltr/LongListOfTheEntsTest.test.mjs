import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/ltr/LongListOfTheEntsTest.java",
  "tests": [
    {
      "name": "test_SimplePlay",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Long List of the Ents",
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
          "zone": "HAND",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Bear Cub",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Woodland Changeling",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Centaur Courser",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Long List of the Ents"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Construct"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Augmenting Automaton"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Bear"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Warrior"
        },
        {
          "op": "castSpell",
          "turn": 5,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Bear Cub"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Centaur"
        },
        {
          "op": "castSpell",
          "turn": 7,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Centaur Courser"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Druid"
        },
        {
          "op": "castSpell",
          "turn": 9,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Balduvian Bears"
        },
        {
          "op": "castSpell",
          "turn": 9,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Llanowar Elves"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Elf"
        },
        {
          "op": "castSpell",
          "turn": 11,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Woodland Changeling"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 12,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Memnite",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Augmenting Automaton",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Grizzly Bears",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Bear Cub",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Centaur Courser",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Balduvian Bears",
          "counter": "P1P1",
          "count": 0
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Llanowar Elves",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Woodland Changeling",
          "counter": "P1P1",
          "count": 1
        }
      ]
    }
  ]
});
