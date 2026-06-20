import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/dynamicvalue/PartyCountTest.java",
  "tests": [
    {
      "name": "testNoMembers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    },
    {
      "name": "testSingleMember",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        }
      ]
    },
    {
      "name": "testSingleMember2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric Wizard",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        }
      ]
    },
    {
      "name": "testTwoMembers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt2",
          "custom": true,
          "typeLine": "Creature - Warrior",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "testTwoMembers2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt2",
          "custom": true,
          "typeLine": "Creature - Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        }
      ]
    },
    {
      "name": "testThreeMembers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt2",
          "custom": true,
          "typeLine": "Creature - Warrior",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt3",
          "custom": true,
          "typeLine": "Creature - Wizard",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "testThreeMembers2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric Warrior",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt2",
          "custom": true,
          "typeLine": "Creature - Cleric Warrior",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt3",
          "custom": true,
          "typeLine": "Creature - Cleric Warrior Wizard",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    },
    {
      "name": "testOddCombos",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Rogue Wizard Warrior",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt2",
          "custom": true,
          "typeLine": "Creature - Rogue Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt3",
          "custom": true,
          "typeLine": "Creature - Cleric Wizard",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "crt4",
          "custom": true,
          "typeLine": "Creature - Warrior Wizard",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 24
        }
      ]
    },
    {
      "name": "testOpponent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "crt1",
          "custom": true,
          "typeLine": "Creature - Cleric",
          "manaCost": "{1}",
          "oracleText": ""
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 21
        }
      ]
    },
    {
      "name": "testChangelings",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "typeLine": "Enchantment",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: You gain life equal to the number of creatures in your party."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Impostor of the Sixth Pride",
          "count": 3
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_TURN"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 23
        }
      ]
    }
  ]
});
