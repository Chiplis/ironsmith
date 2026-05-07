import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/pip/Vault13DwellersJourneyTest.java",
  "tests": [
    {
      "name": "test_SimplePlay_ReturnOne",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vault 13: Dweller's Journey",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vault 13: Dweller's Journey"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "TestPlayer.TARGET_SKIP"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "test_SimplePlay_Return",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vault 13: Dweller's Journey",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vault 13: Dweller's Journey"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 5,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 5,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    },
    {
      "name": "test_SimplePlay_NoReturn",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Vault 13: Dweller's Journey",
          "count": 1
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
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Vampire Hexmage",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Vault 13: Dweller's Journey"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Memnite"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Ornithopter"
        },
        {
          "op": "activateAbility",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "Sacrifice"
        },
        {
          "op": "addTarget",
          "player": 1,
          "target": "Vault 13: Dweller's Journey"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Squire"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Watchwolf"
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 3,
          "phase": "POSTCOMBAT_MAIN",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mountain"
        },
        {
          "op": "assertExileCount",
          "turn": 7,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 7,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 7,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "turn": 7,
          "phase": "UPKEEP",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Memnite^Ornithopter"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 7,
          "phase": "END_TURN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Memnite",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Ornithopter",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 0,
          "name": "Squire",
          "count": 1
        },
        {
          "op": "assertLibraryCount",
          "player": 1,
          "name": "Watchwolf",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        }
      ]
    }
  ]
});
