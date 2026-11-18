import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/emblems/EmblemOfCardTest.java",
  "tests": [
    {
      "name": "testEmblemOfGriselbrand",
      "operations": [
        {
          "op": "addEmblem",
          "player": 0,
          "abilitySource": "Griselbrand",
          "name": "Griselbrand Emblem"
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "Pay 7 life: Draw",
          "source": "Griselbrand Emblem"
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
          "op": "assertHandCount",
          "player": 0,
          "count": 7
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 13
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Griselbrand Emblem",
          "count": 1
        }
      ]
    },
    {
      "name": "testEmblemOfYurlok",
      "operations": [
        {
          "op": "addEmblem",
          "player": 0,
          "abilitySource": "Yurlok of Scorch Thrash",
          "name": "Yurlok of Scorch Thrash Emblem"
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "assertEmblemCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Yurlok of Scorch Thrash Emblem",
          "count": 1
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}",
          "count": 1
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "colors": "R",
          "amount": 1
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "{1}, {T}:",
          "expected": false
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "assertManaPool",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "colors": "R",
          "amount": 0
        },
        {
          "op": "assertLife",
          "turn": 1,
          "phase": "BEGIN_COMBAT",
          "player": 0,
          "life": 19
        },
        {
          "op": "execute"
        }
      ]
    },
    {
      "name": "testEmblemOfOmniscience",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addEmblem",
          "player": 0,
          "abilitySource": "Omniscience",
          "name": "Omniscience Emblem"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Colossal Dreadmaw",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Colossal Dreadmaw"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Cast without paying its mana cost (source: Omniscience"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Colossal Dreadmaw",
          "count": 1
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Omniscience Emblem",
          "count": 1
        }
      ]
    },
    {
      "name": "testEmblemOfParadoxEngine",
      "operations": [
        {
          "op": "addEmblem",
          "player": 0,
          "abilitySource": "Paradox Engine",
          "name": "Paradox Engine Emblem"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mox Emerald",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Basalt Monolith",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Book of Rass",
          "count": 1
        },
        {
          "op": "setLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sol Ring"
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
          "name": "Basalt Monolith"
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
          "name": "Book of Rass"
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
          "ability": "{2}, Pay"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, Pay"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{2}, Pay"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
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
          "op": "assertLife",
          "player": 0,
          "life": 14
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Paradox Engine Emblem",
          "count": 1
        }
      ]
    },
    {
      "name": "testEmblemOfDoublingSeason",
      "operations": [
        {
          "op": "addEmblem",
          "player": 0,
          "abilitySource": "Doubling Season",
          "name": "Doubling Season Emblem"
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Elspeth, Sun's Champion",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elspeth, Sun's Champion"
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elspeth, Sun's Champion",
          "counter": "LOYALTY",
          "count": 8
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "+1: Create"
        },
        {
          "op": "assertPlayableAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "label": "Pay 7 life:",
          "expected": false
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": null
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Elspeth, Sun's Champion",
          "counter": "LOYALTY",
          "count": 9
        },
        {
          "op": "assertPermanentCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Soldier Token",
          "count": 6
        },
        {
          "op": "execute"
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Doubling Season Emblem",
          "count": 1
        }
      ]
    },
    {
      "name": "testEmblemOfMaelstromNexus",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addEmblem",
          "player": 0,
          "abilitySource": "Maelstrom Nexus",
          "name": "Maelstrom Nexus Emblem"
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 2
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Grizzly Bears"
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
          "player": null
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "assertEmblemCount",
          "player": 0,
          "name": "Maelstrom Nexus Emblem",
          "count": 1
        }
      ]
    }
  ]
});
