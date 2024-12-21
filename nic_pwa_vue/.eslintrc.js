module.exports = {
  root: true,
  env: {
    // browser: true,
    node: true,
  },
  extends: [
    "plugin:vue/vue3-essential",
    "eslint:recommended",
    // "plugin:prettier/recommended",
  ],
  parserOptions: {
    parser: "@babel/eslint-parser",
    parserOptions: {
      "ecmaVersion": 2020,
      "requireConfigFile": false,
      "babelOptions": {
        "presets": ["@babel/preset-env"]
      }
    },
  },
  rules: {
    "no-console": process.env.NODE_ENV === "production" ? "warn" : "off",
    "no-debugger": process.env.NODE_ENV === "production" ? "warn" : "off",
  },
};
