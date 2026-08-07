import js from "@eslint/js";
import tseslint from "typescript-eslint";

// Flat config, mirroring vibecoder/eslint.config.js minus the React plugins.
// Before this file existed, `npm run lint` could not run at all: there was no
// ESLint config anywhere and `eslint` was not even a declared dependency, so
// the extension's TypeScript shipped without ever being linted.
export default tseslint.config(
  { ignores: ["out/**", "node_modules/**", "*.js"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: "module",
    },
    rules: {
      // TypeScript already resolves globals and imports via @types/node and
      // @types/vscode; ESLint's own no-undef only duplicates that and reports
      // false positives for ambient types. This is typescript-eslint's own
      // recommendation for TS sources.
      "no-undef": "off",
      // Same as vibecoder: `_`-prefixed bindings are deliberately unused.
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
      "@typescript-eslint/no-explicit-any": "warn",
      "prefer-const": "error",
      "no-console": ["warn", { allow: ["warn", "error"] }],
    },
  },
);
