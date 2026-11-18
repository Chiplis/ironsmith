import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/modification/CostReduceTest.java",
  "tests": [
    {
      "name": "test_Monohybrid",
      "operations": [
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Mountain",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Forest",
          "count": 8
        },
        {
          "op": "addCard",
          "zone": "BATTLEFIELD",
          "player": 0,
          "name": "Wastes",
          "count": 2
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {R}",
          "count": 8
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {G}",
          "count": 8
        },
        {
          "op": "activateManaAbility",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "ability": "{T}: Add {C}",
          "count": 2
        },
        {
          "op": "assertSpellPriceMatrix",
          "turn": 1,
          "phase": "PRECOMBAT_MAIN",
          "player": 0,
          "cases": [
            {
              "printed": "{C}",
              "reduction": 1,
              "payments": {
                "": "0,C"
              }
            },
            {
              "printed": "{C}{G}",
              "reduction": 1,
              "payments": {
                "": "0,C,G"
              }
            },
            {
              "printed": "",
              "reduction": 1,
              "payments": {
                "": "0"
              }
            },
            {
              "printed": "{R}",
              "reduction": 1,
              "payments": {
                "": "0,R"
              }
            },
            {
              "printed": "{R}{G}",
              "reduction": 1,
              "payments": {
                "": "0,G,R"
              }
            },
            {
              "printed": "{1}",
              "reduction": 1,
              "payments": {
                "": "0"
              }
            },
            {
              "printed": "{R}{1}",
              "reduction": 1,
              "payments": {
                "": "0,R"
              }
            },
            {
              "printed": "{1}{R}",
              "reduction": 1,
              "payments": {
                "": "0,R"
              }
            },
            {
              "printed": "{1}{R}{G}",
              "reduction": 1,
              "payments": {
                "": "0,G,R"
              }
            },
            {
              "printed": "{R}{1}{G}",
              "reduction": 1,
              "payments": {
                "": "0,G,R"
              }
            },
            {
              "printed": "{R}{G}{1}",
              "reduction": 1,
              "payments": {
                "": "0,G,R"
              }
            },
            {
              "printed": "{2}",
              "reduction": 1,
              "payments": {
                "": "1"
              }
            },
            {
              "printed": "{R}{2}",
              "reduction": 1,
              "payments": {
                "": "1,R"
              }
            },
            {
              "printed": "{2}{R}",
              "reduction": 1,
              "payments": {
                "": "1,R"
              }
            },
            {
              "printed": "{2}{R}{G}",
              "reduction": 1,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{R}{2}{G}",
              "reduction": 1,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{R}{G}{2}",
              "reduction": 1,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{2}",
              "reduction": 2,
              "payments": {
                "": "0"
              }
            },
            {
              "printed": "{3}",
              "reduction": 2,
              "payments": {
                "": "1"
              }
            },
            {
              "printed": "{R}{3}",
              "reduction": 2,
              "payments": {
                "": "1,R"
              }
            },
            {
              "printed": "{3}{R}",
              "reduction": 2,
              "payments": {
                "": "1,R"
              }
            },
            {
              "printed": "{3}{R}{G}",
              "reduction": 2,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{R}{3}{G}",
              "reduction": 2,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{R}{G}{3}",
              "reduction": 2,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{2}{2}",
              "reduction": 2,
              "payments": {
                "": "2"
              }
            },
            {
              "printed": "{3}{3}",
              "reduction": 2,
              "payments": {
                "": "4"
              }
            },
            {
              "printed": "{3}{R}{3}",
              "reduction": 2,
              "payments": {
                "": "4,R"
              }
            },
            {
              "printed": "{3}{R}{3}{G}",
              "reduction": 2,
              "payments": {
                "": "4,G,R"
              }
            },
            {
              "printed": "{R}{3}{G}{3}",
              "reduction": 2,
              "payments": {
                "": "4,G,R"
              }
            },
            {
              "printed": "{2}{2}",
              "reduction": 3,
              "payments": {
                "": "1"
              }
            },
            {
              "printed": "{3}{3}",
              "reduction": 3,
              "payments": {
                "": "3"
              }
            },
            {
              "printed": "{3}{R}{3}",
              "reduction": 5,
              "payments": {
                "": "1,R"
              }
            },
            {
              "printed": "{3}{R}{3}{G}",
              "reduction": 5,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{R}{3}{G}{3}",
              "reduction": 5,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{C}",
              "reduction": -1,
              "payments": {
                "": "1,C"
              }
            },
            {
              "printed": "{C}{G}",
              "reduction": -1,
              "payments": {
                "": "1,C,G"
              }
            },
            {
              "printed": "",
              "reduction": -1,
              "payments": {
                "": "1"
              }
            },
            {
              "printed": "{R}",
              "reduction": -1,
              "payments": {
                "": "1,R"
              }
            },
            {
              "printed": "{R}{G}",
              "reduction": -1,
              "payments": {
                "": "1,G,R"
              }
            },
            {
              "printed": "{1}",
              "reduction": -1,
              "payments": {
                "": "2"
              }
            },
            {
              "printed": "{R}{1}",
              "reduction": -1,
              "payments": {
                "": "2,R"
              }
            },
            {
              "printed": "{1}{R}",
              "reduction": -1,
              "payments": {
                "": "2,R"
              }
            },
            {
              "printed": "{1}{R}{G}",
              "reduction": -1,
              "payments": {
                "": "2,G,R"
              }
            },
            {
              "printed": "{R}{1}{G}",
              "reduction": -1,
              "payments": {
                "": "2,G,R"
              }
            },
            {
              "printed": "{R}{G}{1}",
              "reduction": -1,
              "payments": {
                "": "2,G,R"
              }
            },
            {
              "printed": "{2}",
              "reduction": -1,
              "payments": {
                "": "3"
              }
            },
            {
              "printed": "{R}{2}",
              "reduction": -1,
              "payments": {
                "": "3,R"
              }
            },
            {
              "printed": "{2}{R}",
              "reduction": -1,
              "payments": {
                "": "3,R"
              }
            },
            {
              "printed": "{2}{R}{G}",
              "reduction": -1,
              "payments": {
                "": "3,G,R"
              }
            },
            {
              "printed": "{R}{2}{G}",
              "reduction": -1,
              "payments": {
                "": "3,G,R"
              }
            },
            {
              "printed": "{R}{G}{2}",
              "reduction": -1,
              "payments": {
                "": "3,G,R"
              }
            },
            {
              "printed": "{3}",
              "reduction": -2,
              "payments": {
                "": "5"
              }
            },
            {
              "printed": "{R}{3}",
              "reduction": -2,
              "payments": {
                "": "5,R"
              }
            },
            {
              "printed": "{3}{R}",
              "reduction": -2,
              "payments": {
                "": "5,R"
              }
            },
            {
              "printed": "{3}{R}{G}",
              "reduction": -2,
              "payments": {
                "": "5,G,R"
              }
            },
            {
              "printed": "{R}{3}{G}",
              "reduction": -2,
              "payments": {
                "": "5,G,R"
              }
            },
            {
              "printed": "{R}{G}{3}",
              "reduction": -2,
              "payments": {
                "": "5,G,R"
              }
            },
            {
              "printed": "{2/R}",
              "reduction": 1,
              "payments": {
                "2": "1",
                "R": "0,R"
              }
            },
            {
              "printed": "{2/R}{2/G}",
              "reduction": 1,
              "payments": {
                "2,2": "3",
                "2,G": "1,G",
                "R,2": "1,R",
                "R,G": "0,G,R"
              }
            },
            {
              "printed": "{2/R}",
              "reduction": -1,
              "payments": {
                "2": "3",
                "R": "1,R"
              }
            },
            {
              "printed": "{2/R}{2/G}",
              "reduction": -1,
              "payments": {
                "2,2": "5",
                "2,G": "3,G",
                "R,2": "3,R",
                "R,G": "1,G,R"
              }
            },
            {
              "printed": "{2/R}{1}",
              "reduction": 1,
              "payments": {
                "2": "2",
                "R": "0,R"
              }
            },
            {
              "printed": "{2/R}{2/G}{1}",
              "reduction": 1,
              "payments": {
                "2,2": "4",
                "2,G": "2,G",
                "R,2": "2,R",
                "R,G": "0,G,R"
              }
            },
            {
              "printed": "{2/R}{1}",
              "reduction": -1,
              "payments": {
                "2": "4",
                "R": "2,R"
              }
            },
            {
              "printed": "{2/R}{2/G}{1}",
              "reduction": -1,
              "payments": {
                "2,2": "6",
                "2,G": "4,G",
                "R,2": "4,R",
                "R,G": "2,G,R"
              }
            },
            {
              "printed": "{2/R}{2}",
              "reduction": 1,
              "payments": {
                "2": "3",
                "R": "1,R"
              }
            },
            {
              "printed": "{2/R}{2/G}{2}",
              "reduction": 1,
              "payments": {
                "2,2": "5",
                "2,G": "3,G",
                "R,2": "3,R",
                "R,G": "1,G,R"
              }
            },
            {
              "printed": "{2/R}{2}",
              "reduction": -1,
              "payments": {
                "2": "5",
                "R": "3,R"
              }
            },
            {
              "printed": "{2/R}{2/G}{2}",
              "reduction": -1,
              "payments": {
                "2,2": "7",
                "2,G": "5,G",
                "R,2": "5,R",
                "R,G": "3,G,R"
              }
            },
            {
              "printed": "{2/R}{1}",
              "reduction": 2,
              "payments": {
                "2": "1",
                "R": "0,R"
              }
            },
            {
              "printed": "{2/R}{2/G}{1}",
              "reduction": 2,
              "payments": {
                "2,2": "3",
                "2,G": "1,G",
                "R,2": "1,R",
                "R,G": "0,G,R"
              }
            },
            {
              "printed": "{2/R}{1}",
              "reduction": -2,
              "payments": {
                "2": "5",
                "R": "3,R"
              }
            },
            {
              "printed": "{2/R}{2/G}{1}",
              "reduction": -2,
              "payments": {
                "2,2": "7",
                "2,G": "5,G",
                "R,2": "5,R",
                "R,G": "3,G,R"
              }
            },
            {
              "printed": "{2}{2/R}{2/G}",
              "reduction": 3,
              "payments": {
                "2,2": "3",
                "2,G": "1,G",
                "R,2": "1,R",
                "R,G": "0,G,R"
              }
            },
            {
              "printed": "{2}{2/R}{2/G}",
              "reduction": 4,
              "payments": {
                "2,2": "2",
                "2,G": "0,G",
                "R,2": "0,R",
                "R,G": "0,G,R"
              }
            },
            {
              "printed": "{2}{2/R}{2/G}",
              "reduction": 5,
              "payments": {
                "2,2": "1",
                "2,G": "0,G",
                "R,2": "0,R",
                "R,G": "0,G,R"
              }
            }
          ]
        }
      ]
    }
  ]
});
