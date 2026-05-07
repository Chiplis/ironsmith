import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/clb/LaezelsAcrobaticsTest.java",
  "tests": [
    {
      "name": "testRollLow",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auramancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brazen Dwarf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Attended Knight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lae'zel's Acrobatics",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Nimbus Wings",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Attended Knight"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 9)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lae'zel's Acrobatics"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Attended Knight",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Auramancer",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Brazen Dwarf",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, create"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nimbus Wings"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nimbus Wings",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Nimbus Wings",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 2
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
        }
      ]
    },
    {
      "name": "testRollHigh",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Auramancer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Brazen Dwarf",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Attended Knight",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Lae'zel's Acrobatics",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Nimbus Wings",
          "count": 2
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Attended Knight"
        },
        {
          "op": "unsupported",
          "source": "setDieRollResult(playerA, 10)"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Lae'zel's Acrobatics"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, create"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nimbus Wings"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Attended Knight",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Auramancer",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Brazen Dwarf",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, create"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nimbus Wings"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nimbus Wings",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Nimbus Wings",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Soldier Token",
          "count": 3
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
        }
      ]
    }
  ]
});
