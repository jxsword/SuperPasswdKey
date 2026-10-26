// eslint flat config（pnpm lint）
import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  { ignores: ["dist/**", "node_modules/**"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      // SP-F-707 纪律辅助：禁止任何 storage 写入路径出现
      "no-restricted-properties": [
        "error",
        { object: "chrome", property: "storage", message: "机密不得进 storage (SP-F-707)" },
      ],
    },
  },
);
