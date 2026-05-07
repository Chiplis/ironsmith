import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mh2/GarthOneEyeTest.java",
  "tests": [
    {
      "name": "testDisenchant",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Garth One-Eye",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Disenchant"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nyxborn Courser"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose"
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
          "op": "unsupported",
          "source": "assertTapped(garth, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Plains\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Disenchant",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 1
        }
      ]
    },
    {
      "name": "testBraingeyser",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Island",
          "count": 5
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Garth One-Eye",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Braingeyser"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "X=3"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": 0
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose"
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
          "op": "unsupported",
          "source": "assertTapped(garth, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Island\", true)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Braingeyser",
          "count": 0
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "count": 0,
          "name": 3
        }
      ]
    },
    {
      "name": "testTerror",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Swamp",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Garth One-Eye",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Terror"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nyxborn Courser"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose"
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
          "op": "unsupported",
          "source": "assertTapped(garth, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Swamp\", true)"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Terror",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 1
        }
      ]
    },
    {
      "name": "testShivanDragon",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Garth One-Eye",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Shivan Dragon"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose"
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
          "source": "assertTapped(garth, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Mountain\", true)"
        }
      ]
    },
    {
      "name": "testRegrowth",
      "operations": [
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
          "name": "Garth One-Eye",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Regrowth"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Nyxborn Courser"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose"
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
          "op": "unsupported",
          "source": "assertTapped(garth, true)"
        },
        {
          "op": "unsupported",
          "source": "assertTapped(\"Forest\", true)"
        },
        {
          "op": "assertHandCount",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Regrowth",
          "count": 0
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "name": "Nyxborn Courser",
          "count": 0
        }
      ]
    },
    {
      "name": "testBlackLotus",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Garth One-Eye",
          "count": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Black Lotus"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Choose"
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
          "source": "assertTapped(garth, true)"
        }
      ]
    }
  ]
});
