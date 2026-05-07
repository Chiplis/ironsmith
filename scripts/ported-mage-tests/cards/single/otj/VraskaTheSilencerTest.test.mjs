import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/otj/VraskaTheSilencerTest.java",
  "tests": [
    {
      "name": "test_CorrectTypes",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Vraska, the Silencer",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Doom Blade",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Red Herring",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Soul Warden",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Doom Blade",
          "target": "Red Herring"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "BEGIN_COMBAT"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Red Herring",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Red Herring\", true)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Red Herring\", CardType.CREATURE, false)"
        },
        {
          "op": "unsupported",
          "source": "assertType(\"Red Herring\", CardType.ARTIFACT, true)"
        },
        {
          "op": "unsupported",
          "source": "assertNotSubtype(\"Red Herring\", SubType.FISH)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Red Herring\", SubType.CLUE)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Red Herring\", SubType.TREASURE)"
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 20
        }
      ]
    }
  ]
});
