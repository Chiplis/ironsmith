import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mom/InvasionOfAlaraTest.java",
  "tests": [
    {
      "name": "testSiegeAndSorcery",
      "operations": [
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Alara",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Composite Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Bottle Golems",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Baloth Pup",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Kalonian Behemoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Lone Missionary",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Stonework Puma",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Meteor Golem",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Bottle Golems",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Alara"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Vampire Hexmage"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "assertGraveyardCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Composite Golem",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Invasion of Alara",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Vampire Hexmage",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "assertHandCount",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice",
          "target": "Invasion of Alara"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Bottle Golems"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Meteor Golem"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Lone Missionary"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, meteor, 1)"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, baloth, 1)"
        },
        {
          "op": "unsupported",
          "source": "addTargetAmount(playerA, behemoth, 1)"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "When {this} enters, you gain"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Glorious Anthem"
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
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 24
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Baloth Pup",
          "ability": "Trample",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Baloth Pup",
          "power": 4,
          "toughness": 2
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Kalonian Behemoth",
          "power": 10,
          "toughness": 10
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Meteor Golem",
          "power": 4,
          "toughness": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Lone Missionary",
          "count": 2
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Invasion of Alara",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Glorious Anthem",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Bottle Golems",
          "count": 1
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Stonework Puma",
          "count": 1
        }
      ]
    }
  ]
});
