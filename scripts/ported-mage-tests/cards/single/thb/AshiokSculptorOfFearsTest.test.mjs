import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/thb/AshiokSculptorOfFearsTest.java",
  "tests": [
    {
      "name": "test_Minus11",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Ashiok, Sculptor of Fears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Doubling Season",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Underground Sea",
          "count": 6
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears@bob1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grizzly Bears@bob2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Taiga",
          "count": 2
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Grist, the Hunger Tide",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Grizzly Bears@carol1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerC",
          "name": "Grizzly Bears@carol2",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Grizzly Bears@dan1",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": "playerD",
          "name": "Grizzly Bears@dan2",
          "count": 1
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ashiok, Sculptor of Fears",
          "resolve": true
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Doubling Season"
        },
        {
          "op": "assertCounterCount",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Ashiok, Sculptor of Fears",
          "counter": "loyalty",
          "count": 16
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "-11",
          "target": 1,
          "source": "Ashiok, Sculptor of Fears"
        },
        {
          "op": "assertStackObject",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "source": "Ashiok, Sculptor of Fears",
          "count": 1,
          "abilityKind": "Activated",
          "targetCount": 1
        },
        {
          "op": "waitStackResolved",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "resolutions": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
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
          "count": 11
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Grizzly Bears",
          "count": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "count": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 1,
          "name": "Grizzly Bears",
          "count": 0
        },
        {
          "op": "assertPermanentCount",
          "name": "Grizzly Bears",
          "count": 2,
          "player": 2
        },
        {
          "op": "assertPermanentCount",
          "name": "Grizzly Bears",
          "count": 2,
          "player": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "@bob1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "@bob2",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 2,
          "name": "@carol1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 2,
          "name": "@carol2",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 3,
          "name": "@dan1",
          "count": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 3,
          "name": "@dan2",
          "count": 1
        }
      ],
      "turnOrder": [
        0,
        3,
        2,
        1
      ],
      "rangeOfInfluence": 1
    }
  ]
});
