import tsParser from "@typescript-eslint/parser"
import tsPlugin from "@typescript-eslint/eslint-plugin"
import reactHooks from "eslint-plugin-react-hooks"

// Configuración "flat" de ESLint 9. El repo usaba `.eslintrc.json` (formato
// legacy), que ESLint 9 ya no lee; por eso movemos la configuración aquí.
export default [
  {
    ignores: ["dist/**", "node_modules/**", "target/**", "src-tauri/**", "graphify-out/**"],
  },
  {
    files: ["src/**/*.{ts,tsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: "latest",
        sourceType: "module",
        ecmaFeatures: { jsx: true },
      },
    },
    plugins: {
      "@typescript-eslint": tsPlugin,
      "react-hooks": reactHooks,
    },
    rules: {
      ...tsPlugin.configs.recommended.rules,
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", caughtErrorsIgnorePattern: "^_" },
      ],
      "no-debugger": "warn",
      "no-empty": ["warn", { allowEmptyCatch: true }],
    },
  },
]
