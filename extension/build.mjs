// 构建脚本（pnpm build）：bundle 三个入口 + 拷贝静态资源到 dist/
// 运行时零 npm 依赖（供应链纪律，design_docs/10 §2）
import * as esbuild from "esbuild";
import { copyFileSync, mkdirSync } from "node:fs";

mkdirSync("dist", { recursive: true });

// MV3 content script 必须为 classic script（iife）；SW/popup 同理用 iife 免 module 声明
// 输出扁平命名与 manifest.json 路径一致：content.js / service-worker.js / popup.js
await esbuild.build({
  entryPoints: [
    { in: "src/content/main.ts", out: "content" },
    { in: "src/background/service-worker.ts", out: "service-worker" },
    { in: "src/popup/popup.ts", out: "popup" },
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
