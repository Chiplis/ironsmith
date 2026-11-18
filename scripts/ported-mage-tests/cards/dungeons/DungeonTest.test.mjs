import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/dungeons/DungeonTest.java",
  "tests": [
    {
      "name": "test__LostMineOfPhandelver_room1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Cave Entrance"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 0
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
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test__LostMineOfPhandelver_room2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Goblin Lair"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 1
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
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test__LostMineOfPhandelver_room3",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Dark Pool"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
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
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test__LostMineOfPhandelver_room4",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
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
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "test__LostMineOfPhandelver_multipleTurns",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
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
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        }
      ]
    },
    {
      "name": "test__LostMineOfPhandelver_rollback",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "Artifact",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
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
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Goblin Lair"
        },
        {
          "op": "saveRuntimeCheckpoint",
          "turn": 2,
          "phase": "UPKEEP",
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Goblin Lair"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Goblin Lair"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 1
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
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test__LostMineOfPhandelver_rollbackDifferentChoice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "Artifact",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
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
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Goblin Lair"
        },
        {
          "op": "saveRuntimeCheckpoint",
          "turn": 2,
          "phase": "UPKEEP",
          "seatIndependent": true
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "POSTCOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 21
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "restoreRuntimeCheckpoint"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Goblin Lair"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "ability": "{0}:",
          "source": "tester",
          "resolve": true,
          "player": 0
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Storeroom"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Silvercoat Lion"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Storeroom"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Silvercoat Lion",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Silvercoat Lion",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 1
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
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test__Dungeon_multiplePlayers",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Dungeon of the Mad Mage"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{0}:"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Goblin Bazaar"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": null
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
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Dark Pool"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 1,
          "dungeon": "Dungeon of the Mad Mage",
          "room": "Lost Level"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertPowerToughness",
          "player": 1,
          "name": "Flamespeaker Adept",
          "power": 6,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Treasure Token",
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
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 1,
          "count": 0
        }
      ]
    },
    {
      "name": "test__CompletedDungeonCondition_true",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gloom Stalker",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gloom Stalker",
          "ability": "DoubleStrike",
          "expected": true
        }
      ]
    },
    {
      "name": "test__CompletedDungeonCondition_false",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gloom Stalker",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gloom Stalker",
          "ability": "DoubleStrike",
          "expected": false
        }
      ]
    },
    {
      "name": "test__CompletedDungeonCondition_falseThenTrue",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gloom Stalker",
          "count": 1
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "UPKEEP"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gloom Stalker",
          "ability": "DoubleStrike",
          "expected": false
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Gloom Stalker",
          "ability": "DoubleStrike",
          "expected": true
        }
      ]
    },
    {
      "name": "test__CompletedDungeonTriggeredAbility",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Dungeon Crawler",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": null,
          "room": null
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Dungeon Crawler",
          "count": 1
        }
      ]
    },
    {
      "name": "test__HamaPasharRuinSeeker_DoubleController",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Hama Pashar, Ruin Seeker",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Dark Pool"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 6,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
          "count": 2
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    },
    {
      "name": "test__HamaPasharRuinSeeker_DontDoubleOpponent",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "tester",
          "custom": true,
          "fixtureSource": true,
          "typeLine": "",
          "power": null,
          "toughness": null,
          "oracleText": "{0}: Venture into the dungeon."
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Flamespeaker Adept",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hama Pashar, Ruin Seeker",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lost Mine of Phandelver"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Goblin Lair"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{0}:"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Dark Pool"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPlayerDungeon",
          "player": 0,
          "dungeon": "Lost Mine of Phandelver",
          "room": "Dark Pool"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Flamespeaker Adept",
          "power": 4,
          "toughness": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Goblin Token",
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
          "life": 19
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        }
      ]
    }
  ]
});
