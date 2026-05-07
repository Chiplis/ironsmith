import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/sok/MichikoKondaTest.java",
  "tests": [
    {
      "name": "testOppDealsToYou",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Michiko Konda, Truth Seeker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mayhem Devil",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cunning Sparkmage",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: {this} deals 1 damage to any target",
          "target": 0
        },
        {
          "op": "setChoice",
          "player": 1,
          "value": "Barony Vampire"
        },
        {
          "op": "addTarget",
          "player": 0,
          "target": "Mayhem Devil"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 19
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertGraveyardCount",
          "player": 0,
          "count": 1,
          "name": 1
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Michiko Konda, Truth Seeker",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, devil, 1)"
        },
        {
          "op": "assertGraveyardCount",
          "player": 1,
          "name": "Barony Vampire",
          "count": 1
        }
      ]
    },
    {
      "name": "testOppDealsToYours",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Michiko Konda, Truth Seeker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mayhem Devil",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Cunning Sparkmage",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 1,
          "ability": "{T}: {this} deals 1 damage to any target",
          "target": "Walking Corpse"
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 20
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 3
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 2
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "name": "Michiko Konda, Truth Seeker",
          "count": 1
        },
        {
          "op": "unsupported",
          "source": "assertDamageReceived(playerA, zombie, 1)"
        }
      ]
    },
    {
      "name": "testYouDealToYourself",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Michiko Konda, Truth Seeker",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mayhem Devil",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Walking Corpse",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 1,
          "name": "Barony Vampire",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Elves of Deep Shadow",
          "count": 1
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {B}. {this} deals 1 damage to you."
        },
        {
          "op": "setStrictChooseMode",
          "value": true
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
          "op": "assertLife",
          "player": 0,
          "life": 19
        },
        {
          "op": "assertLife",
          "player": 1,
          "life": 20
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 0,
          "name": 4
        },
        {
          "op": "assertPermanentCount",
          "player": 0,
          "count": 1,
          "name": 1
        }
      ]
    }
  ]
});
