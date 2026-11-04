/// <reference types="vitest/config" />
import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import pkg from "./package.json" with { type: "json" };

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // Relative asset paths, resolved against the <base href> that
  // `advent build` writes into every index.html: the same files then work at
  // a domain root, under a sub-path, and in every /<event>/ folder.
  base: "./",
  // The engine version for the "Built with Advent" credit: release-prepare
  // bumps package.json alongside Cargo.toml.
  define: { __ADVENT_VERSION__: JSON.stringify(pkg.version) },
  resolve: {
    alias: { "@": path.resolve(import.meta.dirname, "./src") },
  },
  build: {
    outDir: "dist",
  },
  test: {
    globals: true,
    environment: "jsdom",
    setupFiles: "./src/test/setup.ts",
    css: false,
  },
});
