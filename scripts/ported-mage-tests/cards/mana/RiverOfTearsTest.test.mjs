import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/mana/RiverOfTearsTest.java",
  "tests": [
    {
      "name": "testBlackAfterPlayed",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Nightshade Stinger",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "River of Tears"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "POSTCOMBAT_MAIN",
          "player": 0,
          "name": "Nightshade Stinger"
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
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"River of Tears\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nightshade Stinger",
          "count": 1
        }
      ]
    },
    {
      "name": "testBlueInSecondTurn",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Aven Envoy",
          "count": 1
        },
        {
          "op": "playLand",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "River of Tears"
        },
        {
          "op": "castSpell",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Aven Envoy"
        },
        {
          "op": "setStopAt",
          "turn": 3,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "River of Tears",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"River of Tears\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Aven Envoy",
          "count": 1
        }
      ]
    }
  ]
});
