import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/znr/AshayaSoulOfTheWildTest.java",
  "tests": [
    {
      "name": "testAshaya",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ashaya, Soul of the Wild"
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
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "new GreenManaAbility()",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "power": 7,
          "toughness": 7
        },
        {
          "op": "unsupported",
          "source": "assertType(bear, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(bear, CardType.CREATURE, SubType.BEAR)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "new GreenManaAbility()",
          "expected": true
        }
      ]
    },
    {
      "name": "testAshayaNoAbilities",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Kenrith's Transformation",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 7
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ashaya, Soul of the Wild"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Kenrith's Transformation",
          "target": "Ashaya, Soul of the Wild"
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
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.LAND, false)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(ashaya, SubType.ELEMENTAL)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(ashaya, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.CREATURE, SubType.ELK)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "new GreenManaAbility()",
          "expected": false
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "power": 3,
          "toughness": 3
        },
        {
          "op": "unsupported",
          "source": "assertType(bear, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(bear, CardType.CREATURE, SubType.BEAR)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "new GreenManaAbility()",
          "expected": true
        }
      ]
    },
    {
      "name": "testAshayaVolrathsShapeshifter",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Volrath's Shapeshifter",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Volrath's Shapeshifter"
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
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(ashaya, CardType.CREATURE, SubType.ELEMENTAL)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "ability": "new GreenManaAbility()",
          "expected": true
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Ashaya, Soul of the Wild",
          "power": 5,
          "toughness": 5
        },
        {
          "op": "unsupported",
          "source": "assertType(bear, CardType.LAND, SubType.FOREST)"
        },
        {
          "op": "unsupported",
          "source": "assertType(bear, CardType.CREATURE, SubType.BEAR)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Grizzly Bears",
          "ability": "new GreenManaAbility()",
          "expected": true
        }
      ]
    }
  ]
});
