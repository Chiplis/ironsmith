import { registerPortedMageTests } from "../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/copy/LazotepConvertTest.java",
  "tests": [
    {
      "name": "testInvastionAmonkhetTransformed",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Badlands",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Mutagen Connoisseur",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Invasion of Amonkhet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Char",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Invasion of Amonkhet"
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
          "name": "Char",
          "target": "Invasion of Amonkhet"
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
          "name": "Mutagen Connoisseur",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Mutagen Connoisseur",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Mutagen Connoisseur\", SubType.ZOMBIE)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(\"Mutagen Connoisseur\", SubType.VEDALKEN)"
        },
        {
          "op": "unsupported",
          "source": "assertColor(playerA,\"Mutagen Connoisseur\",\"BGU\",true)"
        },
        {
          "op": "assertPowerToughness",
          "player": 0,
          "name": "Mutagen Connoisseur",
          "power": 5,
          "toughness": 4
        }
      ]
    }
  ]
});
