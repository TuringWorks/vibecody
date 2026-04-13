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
      // Classic react-hooks rules only — the v7 plugin's recommended config
      // includes experimental React Compiler rules that flag valid patterns
      // in standard React 18 codebases (refs during render, setState-in-effect, etc.)
      "react-hooks/rules-of-hooks": "error",
      "react-hooks/exhaustive-deps": "warn",
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
