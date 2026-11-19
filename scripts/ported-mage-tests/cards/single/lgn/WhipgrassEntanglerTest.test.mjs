import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/single/lgn/WhipgrassEntanglerTest.java",
  "tests": [
    {
      "name": "gainAbilityHimselfAndPay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Whipgrass Entangler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 20
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Whipgrass Entangler"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Whipgrass Entangler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Finish activating mana abilities"
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 3
        }
      ]
    },
    {
      "name": "gainAbilityTwiceAndPay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Whipgrass Entangler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 20
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Whipgrass Entangler"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Whipgrass Entangler"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Whipgrass Entangler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Finish activating mana abilities"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Whipgrass Entangler"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertLife",
          "player": 1,
          "life": 19
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 6
        }
      ]
    },
    {
      "name": "gainAbilityWithTwoClericAndPay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Whipgrass Entangler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma's Devoted",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 20
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Whipgrass Entangler"
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Akroma's Devoted"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Whipgrass Entangler",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Finish activating mana abilities"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Akroma's Devoted",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertLife",
          "player": 1,
          "life": 17
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 8
        }
      ]
    },
    {
      "name": "gainAbilityFromTwoEntanglerAndPay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma's Devoted",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Whipgrass Entangler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Whipgrass Entangler",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swords to Plowshares",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Plains",
          "count": 20
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Akroma's Devoted"
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swords to Plowshares",
          "target": "Whipgrass Entangler",
          "resolve": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Whipgrass Entangler",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Akroma's Devoted"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Akroma's Devoted",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Plains"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Finish activating mana abilities"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Akroma's Devoted"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertTappedCount",
          "name": "Plains",
          "tapped": true,
          "count": 12
        }
      ]
    },
    {
      "name": "gainAbilityWithCloneAndPay",
      "operations": [
        {
          "op": "setStrictChooseMode",
          "value": true
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Akroma's Devoted",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Whipgrass Entangler@original",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Sakashima the Impostor",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "HAND",
          "player": 0,
          "name": "Swords to Plowshares",
          "count": 1
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Tundra",
          "count": 20
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Akroma's Devoted",
          "source": "Whipgrass Entangler"
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
          "name": "Sakashima the Impostor",
          "resolve": true
        },
        {
          "op": "setOptionChoices",
          "kind": "options",
          "player": 0,
          "description": "Choose what to copy as this permanent enters",
          "choices": [
            "Enter as a copy of Whipgrass Entangler"
          ]
        },
        {
          "op": "assertAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Sakashima the Impostor",
          "ability": "Until end of turn",
          "expected": true
        },
        {
          "op": "castSpell",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "name": "Swords to Plowshares",
          "target": "@original",
          "resolve": true
        },
        {
          "op": "activateAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{1}{W}: Until end of turn, ",
          "target": "Akroma's Devoted",
          "source": "Sakashima the Impostor"
        },
        {
          "op": "attack",
          "turn": 1,
          "player": 0,
          "attacker": "Akroma's Devoted",
          "defender": 1
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Tundra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Tundra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Tundra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Activate Tundra"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Finish activating mana abilities"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": "Akroma's Devoted"
        },
        {
          "op": "setChoice",
          "player": 0,
          "value": true
        },
        {
          "op": "setChoice",
          "player": 0,
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
          "op": "assertLife",
          "player": 1,
          "life": 18
        },
        {
          "op": "assertTappedCount",
          "name": "Tundra",
          "tapped": true,
          "count": 13
        }
      ]
    }
  ]
});
