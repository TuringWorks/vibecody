import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    plugins: {
      "react-hooks": reactHooks,
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      // Allow unused vars prefixed with _ (common React pattern)
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      // Allow explicit any in specific cases (Tauri interop, third-party libs)
      "@typescript-eslint/no-explicit-any": "warn",
      // Prefer const assertions
      "prefer-const": "error",
      // No console.log in production (warn, not error — allow console.error/warn)
      "no-console": ["warn", { allow: ["warn", "error"] }],
    },
  },
  {
    ignores: ["dist/", "node_modules/", "src-tauri/"],
  },
);
