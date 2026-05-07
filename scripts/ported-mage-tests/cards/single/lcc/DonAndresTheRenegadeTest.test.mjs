import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lcc/DonAndresTheRenegadeTest.java",
  "tests": [
    {
      "name": "test_FirstAbility",
      "operations": [
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
          "player": 0,
          "name": "Don Andres, the Renegade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Llanowar Elves",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Act of Treason",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elvish Mystic",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Act of Treason",
          "target": "Elvish Mystic"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Llanowar Elves",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Llanowar Elves",
          "ability": "new MenaceAbility()",
          "expected": false
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Llanowar Elves",
          "ability": "Deathtouch",
          "expected": false
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Llanowar Elves\", SubType.PIRATE)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Elvish Mystic",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Elvish Mystic",
          "ability": "new MenaceAbility()",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Elvish Mystic",
          "ability": "Deathtouch",
          "expected": true
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Elvish Mystic\", SubType.PIRATE)"
        }
      ]
    },
    {
      "name": "test_SecondAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Don Andres, the Renegade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dire Fleet Daredevil",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 1,
          "name": "Revitalize",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dire Fleet Daredevil"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Revitalize"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Revitalize"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Treasure Token",
          "count": 2
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Treasure Token\", true)"
        }
      ]
    }
  ]
});
