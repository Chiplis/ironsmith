import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/e10/PithingNeedleTest.java",
  "tests": [
    {
      "name": "TestPithingNeedle",
      "operations": [
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Pithing Needle",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Silvercoat Lion",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": 0,
          "name": "Pillarfield Ox",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Proteus Staff",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Proteus Staff",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Wall of Air",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerC",
          "name": "Wind Drake",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Island",
          "count": 3
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Eager Cadet",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "LIBRARY",
          "player": "playerD",
          "name": "Storm Crow",
          "count": 2
        },
        {
          "op": "skipInitShuffling"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Pithing Needle"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Proteus Staff"
        },
        {
          "op": "activateAbility",
          "turn": 3,
          "phase": "PRECOMBAT_MAIN",
          "player": "playerC",
          "ability": "{2}{U}",
          "target": "Eager Cadet"
        },
        {
          "op": "activateAbility",
          "turn": 4,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{2}{U}",
          "target": "Wall of Air"
        },
        {
          "op": "setStopAt",
          "turn": 4,
          "phase": "END_TURN"
        },
        {
          "op": "unsupported",
          "source": "try { execute(); Assert.fail(\"must throw exception on execute\"); } catch (Throwable e) { if (!e.getMessage().contains(\"Can't find ability to activate command: {2}{U}$target=Wall of Air\")) { Assert.fail(\"Should have thrown an error about PlayerB not being able to use the staff to target Wall of Air, but got:\\n\" + e.getMessage()); } } assertPermanentCount(playerD, \"Eager Cadet\", 0)"
        },
        {
          "op": "assertPermanentCount",
          "name": "Storm Crow",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Wall of Air",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "name": "Wind Drake",
          "count": 0
        }
      ]
    }
  ]
});
