import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  // Relative paths: the build works from any folder (GitHub Pages project site, iframe).
  base: "./",
  plugins: [react()],
  worker: { format: "es" },
});
