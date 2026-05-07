import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

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
          "player": 1,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "value": "ModeChoice.LEFT.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible\", 1, playerA, ancients, playerD, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 1, playerA, ancients, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 1, playerA, ancients, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 1, playerA, ancients, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible\", 2, playerD, devil, playerC, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 2, playerD, devil, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 2, playerD, devil, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 2, playerD, devil, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible -- not in range of Pramikon\", 3, playerC, crocodile, playerB, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 3, playerC, crocodile, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible -- not in range of Pramikon\", 3, playerC, crocodile, playerD, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 3, playerC, crocodile, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible\", 4, playerB, bogstomper, playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 4, playerB, bogstomper, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 4, playerB, bogstomper, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 4, playerB, bogstomper, playerB, false)"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
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
          "player": 1,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "value": "ModeChoice.RIGHT.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 1, playerA, ancients, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 1, playerA, ancients, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible\", 1, playerA, ancients, playerB, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 1, playerA, ancients, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 2, playerD, devil, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 2, playerD, devil, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible\", 2, playerD, devil, playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 2, playerD, devil, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible -- not in range of Pramikon\", 3, playerC, crocodile, playerB, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 3, playerC, crocodile, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible -- not in range of Pramikon\", 3, playerC, crocodile, playerD, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 3, playerC, crocodile, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 4, playerB, bogstomper, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 4, playerB, bogstomper, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible\", 4, playerB, bogstomper, playerC, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 4, playerB, bogstomper, playerB, false)"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
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
          "player": "playerD",
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
          "player": "playerD",
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "player": 1,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "value": "ModeChoice.RIGHT.toString()"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "ModeChoice.LEFT.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 2, playerD, devil, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 2, playerD, devil, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 2, playerD, devil, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 2, playerD, devil, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible -- not in range of A's Pramikon\", 3, playerC, crocodile, playerB, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 3, playerC, crocodile, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 3, playerC, crocodile, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 3, playerC, crocodile, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 4, playerB, bogstomper, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 4, playerB, bogstomper, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible -- not in range of D's Pramikon\", 4, playerB, bogstomper, playerC, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 4, playerB, bogstomper, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 5, playerA, ancients, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 5, playerA, ancients, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 5, playerA, ancients, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 5, playerA, ancients, playerA, false)"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
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
          "player": "playerD",
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
          "player": "playerD",
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Island",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "player": 1,
          "name": "Bogstomper",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Catacomb Crocodile",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
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
          "value": "ModeChoice.LEFT.toString()"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerD",
          "name": "Pramikon, Sky Rampart"
        },
        {
          "op": "setChoice",
          "player": "playerD",
          "value": "ModeChoice.RIGHT.toString()"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 2, playerD, devil, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 2, playerD, devil, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 2, playerD, devil, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 2, playerD, devil, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 3, playerC, crocodile, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 3, playerC, crocodile, playerA, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right possible -- not in range of A's Pramikon\", 3, playerC, crocodile, playerD, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 3, playerC, crocodile, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left possible -- not in range of D's Pramikon\", 4, playerB, bogstomper, playerA, true)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 4, playerB, bogstomper, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 4, playerB, bogstomper, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 4, playerB, bogstomper, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack left impossible\", 5, playerA, ancients, playerD, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack out of range impossible\", 5, playerA, ancients, playerC, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack right impossible\", 5, playerA, ancients, playerB, false)"
        },
        {
          "op": "unsupported",
          "source": "checkMayAttackDefender(\"Attack self impossible\", 5, playerA, ancients, playerA, false)"
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        }
      ]
    }
  ]
});
