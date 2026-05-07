import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/thb/OneWithTheStarsTest.java",
  "tests": [
    {
      "name": "testDragonsoulKnight",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "One with the Stars",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dragonsoul Knight",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Stars",
          "target": "Dragonsoul Knight"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "ability": "{W}{U}"
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
          "source": "assertType(knight, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(knight, CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(knight, SubType.HUMAN)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(knight, SubType.KNIGHT)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(knight, SubType.DRAGON)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Dragonsoul Knight",
          "ability": "Flying",
          "expected": true
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Dragonsoul Knight",
          "ability": "Trample",
          "expected": true
        }
      ]
    },
    {
      "name": "testGingerbrute",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "One with the Stars",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Gingerbrute",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Stars",
          "target": "Gingerbrute"
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
          "source": "assertType(brute, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(brute, CardType.ARTIFACT, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(brute, CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(brute, SubType.GOLEM)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(brute, SubType.FOOD)"
        }
      ]
    },
    {
      "name": "testShrine",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "One with the Stars",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Honden of Cleansing Fire",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Stars",
          "target": "Honden of Cleansing Fire"
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
          "source": "assertType(shrine, CardType.ENCHANTMENT, SubType.SHRINE)"
        }
      ]
    },
    {
      "name": "testBitterblossom",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "One with the Stars",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Bitterblossom",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "One with the Stars",
          "target": "Bitterblossom"
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
          "source": "assertType(blossom, CardType.ENCHANTMENT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(blossom, CardType.KINDRED, false)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(blossom, SubType.FAERIE)"
        }
      ]
    }
  ]
});
