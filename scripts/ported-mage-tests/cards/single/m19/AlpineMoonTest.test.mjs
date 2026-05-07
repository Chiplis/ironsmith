import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/m19/AlpineMoonTest.java",
  "tests": [
    {
      "name": "testAlpineMoonAfterUrborg",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Alpine Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Urborg, Tomb of Yawgmoth"
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
          "op": "unsupported",
          "source": "assertNotSubtype(urborg, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Mountain\", SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Island\", SubType.SWAMP)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Urborg, Tomb of Yawgmoth",
          "ability": "new AnyColorManaAbility()",
          "expected": true
        }
      ]
    },
    {
      "name": "testAlpineMoonBeforeUrborg",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Urborg, Tomb of Yawgmoth",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Alpine Moon",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Urborg, Tomb of Yawgmoth"
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
          "op": "unsupported",
          "source": "assertNotSubtype(urborg, SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Mountain\", SubType.SWAMP)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Island\", SubType.SWAMP)"
        },
        {
          "op": "assertAbility",
          "player": 1,
          "name": "Urborg, Tomb of Yawgmoth",
          "ability": "new AnyColorManaAbility()",
          "expected": true
        }
      ]
    }
  ]
});
