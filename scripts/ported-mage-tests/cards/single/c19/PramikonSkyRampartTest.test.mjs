import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

// XMage A, D, C, B occupy clockwise seats 0, 1, 2, 3.
registerPortedMageTests({
  "sourcePath": "scripts/cards/single/c19/PramikonSkyRampartTest.java",
  "tests": [
    {
      "name": "chooseLeft",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pramikon, Sky Rampart",
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
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 2,
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hulking Devil",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Left"
        },
        {
          "op": "assertAttackDefenders",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "Indomitable Ancients",
          "defenders": [
            1
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "attacker": "Hulking Devil",
          "defenders": [
            2
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 2,
          "attacker": "Catacomb Crocodile",
          "defenders": [
            3,
            1
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 3,
          "attacker": "Bogstomper",
          "defenders": [
            0
          ]
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        1,
        2,
        3
      ]
    },
    {
      "name": "chooseRight",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pramikon, Sky Rampart",
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
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 2,
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hulking Devil",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Right"
        },
        {
          "op": "assertAttackDefenders",
          "turn": 1,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "Indomitable Ancients",
          "defenders": [
            3
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "attacker": "Hulking Devil",
          "defenders": [
            0
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 2,
          "attacker": "Catacomb Crocodile",
          "defenders": [
            3,
            1
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 3,
          "attacker": "Bogstomper",
          "defenders": [
            2
          ]
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        1,
        2,
        3
      ]
    },
    {
      "name": "doublePramikon",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pramikon, Sky Rampart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Pramikon, Sky Rampart",
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
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 2,
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hulking Devil",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Right"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Left"
        },
        {
          "op": "assertAttackDefenders",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "attacker": "Hulking Devil",
          "defenders": []
        },
        {
          "op": "assertAttackDefenders",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 2,
          "attacker": "Catacomb Crocodile",
          "defenders": [
            3
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 3,
          "attacker": "Bogstomper",
          "defenders": [
            2
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 5,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "Indomitable Ancients",
          "defenders": []
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        1,
        2,
        3
      ]
    },
    {
      "name": "doublePramikonOther",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pramikon, Sky Rampart",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Pramikon, Sky Rampart",
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
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Indomitable Ancients",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 3,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 2,
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hulking Devil",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Left"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Right"
        },
        {
          "op": "assertAttackDefenders",
          "turn": 2,
          "phase": "DECLARE_ATTACKERS",
          "player": 1,
          "attacker": "Hulking Devil",
          "defenders": []
        },
        {
          "op": "assertAttackDefenders",
          "turn": 3,
          "phase": "DECLARE_ATTACKERS",
          "player": 2,
          "attacker": "Catacomb Crocodile",
          "defenders": [
            1
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 4,
          "phase": "DECLARE_ATTACKERS",
          "player": 3,
          "attacker": "Bogstomper",
          "defenders": [
            0
          ]
        },
        {
          "op": "assertAttackDefenders",
          "turn": 5,
          "phase": "DECLARE_ATTACKERS",
          "player": 0,
          "attacker": "Indomitable Ancients",
          "defenders": []
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ],
      "turnOrder": [
        0,
        1,
        2,
        3
      ]
    }
  ],
  "rangeOfInfluence": 1
});
