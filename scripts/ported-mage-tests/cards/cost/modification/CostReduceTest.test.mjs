import { registerPortedMageTests } from "../../../../mage-port-runner.mjs";

registerPortedMageTests({
  "sourcePath": "scripts/cards/cost/modification/CostReduceTest.java",
  "tests": [
    {
      "name": "test_Monohybrid",
      "operations": [
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{5/R}\" + \" after reduction by \" + 0 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{C}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{C}{G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{1}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{1}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{1}{R}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{1}{R}{G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{1}{G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}{1}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{2}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{R}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{R}{G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{2}{G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}{2}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{3}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}{G}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{3}{G}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}{3}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{2}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{3}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}{3}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}{3}{G}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{3}{G}{3}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{2}\" + \" after reduction by \" + 3 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{3}\" + \" after reduction by \" + 3 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}{3}\" + \" after reduction by \" + 5 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}{3}{G}\" + \" after reduction by \" + 5 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{3}{G}{3}\" + \" after reduction by \" + 5 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{C}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{C}{G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{1}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{1}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{1}{R}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{1}{R}{G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{1}{G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}{1}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{2}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{R}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{R}{G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{2}{G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}{2}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{3}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{3}{R}{G}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{3}{G}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{R}{G}{3}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{1}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}{1}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{1}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}{1}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}{2}\" + \" after reduction by \" + 1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}{2}\" + \" after reduction by \" + -1 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{1}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}{1}\" + \" after reduction by \" + 2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{1}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2/R}{2/G}{1}\" + \" after reduction by \" + -2 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{2/R}{2/G}\" + \" after reduction by \" + 3 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{2/R}{2/G}\" + \" after reduction by \" + 4 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        },
        {
          "op": "unsupported",
          "source": "if (!reduced.getText().equals(need.getText())) { Assert.fail(\"{2}{2/R}{2/G}\" + \" after reduction by \" + 5 + \" must be \" + need.getText() + \", but get \" + reduced.getText()); }"
        }
      ]
    }
  ]
});
