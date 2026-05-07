import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/mom/AyaraWidowOfTheRealmTest.java",
  "tests": [
    {
      "name": "testBackside",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Ayara, Widow of the Realm",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "GRAVEYARD",
          "player": 0,
          "name": "Noxious Gearhulk",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 6
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{5}"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": false
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Noxious Gearhulk"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Balduvian Bears"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Noxious Gearhulk",
          "defender": 1
        },
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "setStopAt",
          "turn": 2,
          "phase": "PRECOMBAT_MAIN"
        },
        {
          "op": "execute"
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Ayara, Furnace Queen",
          "count": 1
        },
        {
          "op": "assertExileCount",
          "player": 0,
          "name": "Noxious Gearhulk",
          "count": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Balduvian Bears",
          "count": 1
        },
        {
          "op": "assertLife",
          "player": 0,
          "life": 22
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 15
        }
      ]
    }
  ]
});
