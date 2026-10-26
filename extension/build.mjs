// 构建脚本（pnpm build）：bundle 三个入口 + 拷贝静态资源到 dist/
// 运行时零 npm 依赖（供应链纪律，design_docs/10 §2）
import * as esbuild from "esbuild";
import { copyFileSync, mkdirSync } from "node:fs";

mkdirSync("dist", { recursive: true });

// MV3 content script 必须为 classic script（iife）；SW/popup 同理用 iife 免 module 声明
await esbuild.build({
  entryPoints: [
    "src/content/main.ts",
    "src/background/service-worker.ts",
    "src/popup/popup.ts",
  ],
  bundle: true,
  outdir: "dist",
  format: "iife",
  target: "chrome120",
  sourcemap: false,
  minify: false,
  logLevel: "info",
});

copyFileSync("manifest.json", "dist/manifest.json");
copyFileSync("popup.html", "dist/popup.html");
console.log("build ok -> dist/");
