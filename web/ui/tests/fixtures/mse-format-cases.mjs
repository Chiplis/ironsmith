// Expected results captured from the pinned native CLI; see reports/mse/format-native-reference.json.
export const formatCases=[
  {
    "input": "7",
    "format": "04d",
    "expression": "format(7,format:\"04d\")",
    "expected": "0007"
  },
  {
    "input": "-7",
    "format": "05d",
    "expression": "format(-7,format:\"05d\")",
    "expected": "-0007"
  },
  {
    "input": "7",
    "format": "+5d",
    "expression": "format(7,format:\"+5d\")",
    "expected": "   +7"
  },
  {
    "input": "7",
    "format": " 5d",
    "expression": "format(7,format:\" 5d\")",
    "expected": "    7"
  },
  {
    "input": "7",
    "format": "-5d",
    "expression": "format(7,format:\"-5d\")",
    "expected": "7    "
  },
  {
    "input": "7",
    "format": "5.3d",
    "expression": "format(7,format:\"5.3d\")",
    "expected": "  007"
  },
  {
    "input": "0",
    "format": ".0d",
    "expression": "format(0,format:\".0d\")",
    "expected": ""
  },
  {
    "input": "255",
    "format": "#x",
    "expression": "format(255,format:\"#x\")",
    "expected": "0xff"
  },
  {
    "input": "255",
    "format": "#08X",
    "expression": "format(255,format:\"#08X\")",
    "expected": "0X0000FF"
  },
  {
    "input": "-1",
    "format": "x",
    "expression": "format(-1,format:\"x\")",
    "expected": "ffffffff"
  },
  {
    "input": "9",
    "format": "#o",
    "expression": "format(9,format:\"#o\")",
    "expected": "011"
  },
  {
    "input": "0",
    "format": "#.0o",
    "expression": "format(0,format:\"#.0o\")",
    "expected": "0"
  },
  {
    "input": "1.25",
    "format": ".1f",
    "expression": "format(1.25,format:\".1f\")",
    "expected": "1.2"
  },
  {
    "input": "2.5",
    "format": ".0f",
    "expression": "format(2.5,format:\".0f\")",
    "expected": "2"
  },
  {
    "input": "3.5",
    "format": ".0f",
    "expression": "format(3.5,format:\".0f\")",
    "expected": "4"
  },
  {
    "input": "-2.5",
    "format": ".0f",
    "expression": "format(-2.5,format:\".0f\")",
    "expected": "-2"
  },
  {
    "input": "1.125",
    "format": "08.2f",
    "expression": "format(1.125,format:\"08.2f\")",
    "expected": "00001.12"
  },
  {
    "input": "3.0",
    "format": "#.0f",
    "expression": "format(3.0,format:\"#.0f\")",
    "expected": "3."
  },
  {
    "input": "1234.5",
    "format": ".2e",
    "expression": "format(1234.5,format:\".2e\")",
    "expected": "1.23e+03"
  },
  {
    "input": "0.00125",
    "format": "+.2E",
    "expression": "format(0.00125,format:\"+.2E\")",
    "expected": "+1.25E-03"
  },
  {
    "input": "1234.5",
    "format": ".3g",
    "expression": "format(1234.5,format:\".3g\")",
    "expected": "1.23e+03"
  },
  {
    "input": "0.00001234",
    "format": ".3g",
    "expression": "format(0.00001234,format:\".3g\")",
    "expected": "1.23e-05"
  },
  {
    "input": "123.0",
    "format": "#.5g",
    "expression": "format(123.0,format:\"#.5g\")",
    "expected": "123.00"
  },
  {
    "input": "1.0",
    "format": ".3G",
    "expression": "format(1.0,format:\".3G\")",
    "expected": "1"
  },
  {
    "input": "9.999",
    "format": ".2e",
    "expression": "format(9.999,format:\".2e\")",
    "expected": "1.00e+01"
  },
  {
    "input": "999.9",
    "format": ".3g",
    "expression": "format(999.9,format:\".3g\")",
    "expected": "1e+03"
  },
  {
    "input": "0.00009999",
    "format": ".3g",
    "expression": "format(0.00009999,format:\".3g\")",
    "expected": "0.0001"
  },
  {
    "input": "2.675",
    "format": ".2f",
    "expression": "format(2.675,format:\".2f\")",
    "expected": "2.67"
  },
  {
    "input": "1.005",
    "format": ".2f",
    "expression": "format(1.005,format:\".2f\")",
    "expected": "1.00"
  },
  {
    "input": "0.0",
    "format": "#.3g",
    "expression": "format(0.0,format:\"#.3g\")",
    "expected": "0.00"
  },
  {
    "input": "0.0",
    "format": ".0e",
    "expression": "format(0.0,format:\".0e\")",
    "expected": "0e+00"
  },
  {
    "input": "12.0",
    "format": "010.2e",
    "expression": "format(12.0,format:\"010.2e\")",
    "expected": "001.20e+01"
  },
  {
    "input": "-0.0",
    "format": "+.1f",
    "expression": "format(-0.0,format:\"+.1f\")",
    "expected": "-0.0"
  },
  {
    "input": "123.0",
    "format": ".0g",
    "expression": "format(123.0,format:\".0g\")",
    "expected": "1e+02"
  },
  {
    "input": "7",
    "format": "%04d",
    "expression": "format(7,format:\"%04d\")",
    "expected": "0007"
  },
  {
    "input": "7",
    "format": "%%04d",
    "expression": "format(7,format:\"%%04d\")",
    "expected": "0007"
  },
  {
    "input": "7",
    "format": "04d!",
    "expression": "format(7,format:\"04d!\")",
    "expected": "0007!"
  },
  {
    "input": "15",
    "format": "-#8x",
    "expression": "format(15,format:\"-#8x\")",
    "expected": "0xf     "
  },
  {
    "input": "\"hello\"",
    "format": "8s",
    "expression": "format(\"hello\",format:\"8s\")",
    "expected": "   hello"
  },
  {
    "input": "\"hello\"",
    "format": "-8.3s",
    "expression": "format(\"hello\",format:\"-8.3s\")",
    "expected": "hel     "
  },
  {
    "input": "\"hello\"",
    "format": ".0s",
    "expression": "format(\"hello\",format:\".0s\")",
    "expected": ""
  },
  {
    "input": "\"éclair\"",
    "format": ".3s",
    "expression": "format(\"éclair\",format:\".3s\")",
    "expected": "écl"
  }
];
