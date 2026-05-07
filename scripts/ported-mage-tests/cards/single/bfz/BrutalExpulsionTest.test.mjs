import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/bfz/BrutalExpulsionTest.java",
  "tests": [
    {
      "name": "test_useSecondModeOnCreature",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Brutal Expulsion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Brutal Expulsion",
          "target": "mode=2Augmenting Automaton"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brutal Expulsion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Razorclaw Bear",
          "count": 1
        }
      ]
    },
    {
      "name": "test_useSecondModeOnPlaneswalker",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Brutal Expulsion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kiora, the Crashing Wave",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Brutal Expulsion",
          "target": "mode=2Kiora, the Crashing Wave"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Brutal Expulsion",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kiora, the Crashing Wave",
          "count": 0
        }
      ]
    },
    {
      "name": "test_useSpellOnPlaneswalker",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Kiora, the Crashing Wave",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": "Kiora, the Crashing Wave"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shock",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Augmenting Automaton",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Razorclaw Bear",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Kiora, the Crashing Wave",
          "count": 0
        }
      ]
    },
    {
      "name": "testPlaneswalkerExile",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Brutal Expulsion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
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
          "player": 0,
          "name": "Shock",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gideon, Ally of Zendikar",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Brutal Expulsion",
          "target": "mode=2Gideon, Ally of Zendikar"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "2"
        },
        {
          "op": "setModeChoice",
          "player": 0,
          "value": "TestPlayer.MODE_SKIP"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Shock",
          "target": "Gideon, Ally of Zendikar"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Shock",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gideon, Ally of Zendikar",
          "count": 0
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Gideon, Ally of Zendikar",
          "count": 1
        }
      ]
    }
  ]
});
