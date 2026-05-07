import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/dka/DungeonGeistsTest.java",
  "tests": [
    {
      "name": "testCard",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dungeon Geists"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DRAW"
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
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Dungeon Geists\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Craw Wurm\", true)"
        }
      ]
    },
    {
      "name": "testCard1",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 1,
          "name": "Act of Treason",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 3
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dungeon Geists"
        },
        {
          "op": "castSpell",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "name": "Act of Treason",
          "target": "Dungeon Geists"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "DRAW"
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
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Dungeon Geists\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Craw Wurm\", false)"
        }
      ]
    },
    {
      "name": "testCard2",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 9
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Mind Control",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dungeon Geists"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Mind Control",
          "target": "Craw Wurm"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "DRAW"
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
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Dungeon Geists\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Craw Wurm\", true)"
        }
      ]
    },
    {
      "name": "testWithBlink",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 4
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Cloudshift",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Dungeon Geists"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Craw Wurm"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Cloudshift",
          "target": "Dungeon Geists"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Elite Vanguard"
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "DRAW"
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
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dungeon Geists",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Craw Wurm",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Elite Vanguard",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Dungeon Geists\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Elite Vanguard\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Craw Wurm\", false)"
        }
      ]
    }
  ]
});
