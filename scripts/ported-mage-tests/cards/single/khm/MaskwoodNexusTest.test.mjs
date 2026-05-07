import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/khm/MaskwoodNexusTest.java",
  "tests": [
    {
      "name": "magdaSearch",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Magda, Brazen Outlaw",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Sol Ring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maskwood Nexus",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Pinnacle Monk",
          "count": 1
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
          "name": "Treasure Vault",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{X}{X}, {T}, Sacrifice"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=5"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Treasure Token"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "Sacrifice five Treasures:"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Pinnacle Monk"
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
          "name": "Pinnacle Monk",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Magda, Brazen Outlaw",
          "count": 1
        }
      ]
    },
    {
      "name": "maskwoodAndJadeOrbCounterTest",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pinnacle Monk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Maskwood Nexus",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Jade Orb of Dragonkind",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pinnacle Monk"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "END_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(jadeOrb, true)"
        },
        {
          "op": "assertCounterCount",
          "player": 0,
          "name": "Pinnacle Monk",
          "counter": "P1P1",
          "count": 1
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Pinnacle Monk",
          "ability": "Hexproof",
          "expected": true
        }
      ]
    }
  ]
});
