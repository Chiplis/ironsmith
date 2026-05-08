import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/continuous/BecomesCreatureAttachedTest.java",
  "tests": [
    {
      "name": "test_CreatureLandWithColor",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 1
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dryad Arbor",
          "power": 1,
          "toughness": 1
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Forest\", \"WUBGR\", false)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"G\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"WUBR\", false)"
        }
      ]
    },
    {
      "name": "test_AttachToLandWithColorReplace",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Wind Zendikon",
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
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Wind Zendikon",
          "target": "Dryad Arbor"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dryad Arbor",
          "power": 2,
          "toughness": 2
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.LAND, true)"
        },
        {
          "op": "assertAbilities",
          "player": 0,
          "name": "Dryad Arbor",
          "abilities": [
            "new AbilitiesImpl<>(FlyingAbility.getInstance())"
          ]
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"U\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"WBGR\", false)"
        }
      ]
    },
    {
      "name": "test_AttachToLandWithColorAdd",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Deep Freeze",
          "count": 1
        },
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
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Deep Freeze",
          "target": "Dryad Arbor"
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
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Dryad Arbor",
          "count": 1
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Dryad Arbor",
          "power": 0,
          "toughness": 4
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.CREATURE, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.LAND, true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Dryad Arbor\", CardType.LAND, SubType.WALL)"
        },
        {
          "op": "assertAbilities",
          "player": 0,
          "name": "Dryad Arbor",
          "abilities": [
            "new AbilitiesImpl<>(DefenderAbility.getInstance())"
          ]
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"UG\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA, \"Dryad Arbor\", \"WBR\", false)"
        }
      ]
    }
  ]
});
