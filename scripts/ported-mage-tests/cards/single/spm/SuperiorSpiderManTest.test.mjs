import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/spm/SuperiorSpiderManTest.java",
  "tests": [
    {
      "name": "testSuperiorSpiderMan",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Superior Spider-Man",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Adelbert Steiner",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 4
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Superior Spider-Man"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Adelbert Steiner"
        },
        {
          "op": "setStopAt",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(superiorSpiderMan, SubType.KNIGHT)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(superiorSpiderMan, SubType.SPIDER)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(superiorSpiderMan, SubType.HERO)"
        },
        {
          "op": "unsupported",
          "source": "assertSubtype(superiorSpiderMan, SubType.HUMAN)"
        },
        {
          "op": "assertAbility",
          "player": 0,
          "name": "Superior Spider-Man",
          "ability": "Lifelink",
          "expected": true
        }
      ]
    }
  ]
});
