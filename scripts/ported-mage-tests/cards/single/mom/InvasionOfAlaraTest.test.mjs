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
          "toughness": 4,
          "seatIndependent": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Composite Golem",
          "ability": "Add {W}{U}{B}{R}{G}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "WUBRG",
          "amount": 5
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Composite Golem",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Alara"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Invasion of Alara",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Invasion of Alara",
          "counter": "DEFENSE",
          "count": 7
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Invasion of Alara",
          "count": 1,
          "abilityKind": "Triggered"
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
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Divination",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Vampire Hexmage",
          "power": 2,
          "toughness": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Vampire Hexmage",
          "ability": "Remove all counters",
          "target": "Invasion of Alara"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Vampire Hexmage",
          "count": 1,
          "abilityKind": "Activated",
          "objectTarget": "Invasion of Alara"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Invasion of Alara",
          "count": 1,
          "abilityKind": "Triggered"
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
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Awaken the Maelstrom",
          "count": 0
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Awaken the Maelstrom",
          "count": 1,
          "objectTarget": "Bottle Golems",
          "targetCount": 2
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
          "op": "addTargetAmount",
          "player": 0,
          "target": "Meteor Golem",
          "amount": 1,
          "selectTarget": false
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Baloth Pup",
          "amount": 1,
          "selectTarget": false
        },
        {
          "op": "addTargetAmount",
          "player": 0,
          "target": "Kalonian Behemoth",
          "amount": 1,
          "selectTarget": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Lone Missionary",
          "count": 1,
          "abilityKind": "Triggered"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Meteor Golem",
          "count": 1,
          "abilityKind": "Triggered",
          "completeTargets": true
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "source": "Bottle Golems",
          "count": 1,
          "abilityKind": "Triggered",
          "controller": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Glorious Anthem"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
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
