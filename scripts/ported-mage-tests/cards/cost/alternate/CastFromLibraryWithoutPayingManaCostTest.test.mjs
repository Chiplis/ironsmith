import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/alternate/CastFromLibraryWithoutPayingManaCostTest.java",
  "tests": [
    {
      "name": "testCastCardFromLibraryTwice",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Storm Crow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sunforger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elixir of Immortality",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Utter End@utter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Sunforger",
          "ability": "Equip",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "target": "Storm Crow"
        },
        {
          "op": "assertStackObject",
          "source": "Sunforger",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Storm Crow"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Sunforger\"",
          "target": "Storm Crow",
          "expected": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "Add {R}"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "Add {W}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "W",
          "amount": 1
        },
        {
          "op": "activateAbility",
          "source": "Sunforger",
          "ability": "Search your library",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Sunforger",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Utter End"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Gray Ogre"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Utter End",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Gray Ogre"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Sunforger",
          "target": "Storm Crow",
          "expected": false
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@utter",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Elixir of Immortality",
          "ability": "you gain 5 life",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertStackObject",
          "source": "Elixir of Immortality",
          "count": 1,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elixir of Immortality",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "@utter",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Elixir of Immortality",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Sunforger",
          "ability": "Equip",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Storm Crow"
        },
        {
          "op": "assertStackObject",
          "source": "Sunforger",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Storm Crow"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Sunforger\"",
          "target": "Storm Crow",
          "expected": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "Add {R}"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "Add {W}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "W",
          "amount": 1
        },
        {
          "op": "activateAbility",
          "source": "Sunforger",
          "ability": "Search your library",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Sunforger",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Utter End"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hill Giant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Utter End",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Hill Giant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Sunforger",
          "target": "Storm Crow",
          "expected": false
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@utter",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 0
        }
      ]
    },
    {
      "name": "testCastCardFromHandAndThenFromLibrary",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Storm Crow",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Sunforger",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elixir of Immortality",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 10
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Utter End@utter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "castSpell",
          "name": "Utter End",
          "player": 0,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "objectAlias": "@utter",
          "target": "Gray Ogre"
        },
        {
          "op": "assertStackObject",
          "source": "Utter End",
          "count": 1,
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Gray Ogre"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Gray Ogre",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@utter",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Elixir of Immortality",
          "ability": "you gain 5 life",
          "player": 0,
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertStackObject",
          "source": "Elixir of Immortality",
          "count": 1,
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "abilityKind": "Activated"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 25
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elixir of Immortality",
          "count": 0
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "@utter",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Elixir of Immortality",
          "count": 1
        },
        {
          "op": "activateAbility",
          "source": "Sunforger",
          "ability": "Equip",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "target": "Storm Crow"
        },
        {
          "op": "assertStackObject",
          "source": "Sunforger",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated",
          "objectTarget": "Storm Crow"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "\"Sunforger\"",
          "target": "Storm Crow",
          "expected": true
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Mountain",
          "ability": "Add {R}"
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "source": "Plains",
          "ability": "Add {W}"
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertManaPool",
          "player": 0,
          "colors": "W",
          "amount": 1
        },
        {
          "op": "activateAbility",
          "source": "Sunforger",
          "ability": "Search your library",
          "player": 0,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertStackObject",
          "source": "Sunforger",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "abilityKind": "Activated"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Utter End"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Hill Giant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertStackObject",
          "source": "Utter End",
          "count": 1,
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "isSpell": true,
          "objectTarget": "Hill Giant"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN"
        },
        {
          "op": "assertAttachedTo",
          "player": 0,
          "attachment": "Sunforger",
          "target": "Storm Crow",
          "expected": false
        },
        {
          "op": "assertExileCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "@utter",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Gray Ogre",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Hill Giant",
          "count": 0
        }
      ]
    }
  ]
});
