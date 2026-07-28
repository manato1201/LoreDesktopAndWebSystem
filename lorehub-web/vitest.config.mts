import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Scoped to pure-logic and fetch-mocking unit tests (see AGENTS.md — this
// Next.js version's async Server Components aren't renderable under Vitest
// anyway, per node_modules/next/dist/docs/01-app/02-guides/testing/vitest.md).
// `node` environment is enough since nothing here touches the DOM; the React
// plugin is only needed so importing a ".tsx" component module (e.g. to
// reach a co-located pure helper) can be JSX-transformed.
export default defineConfig({
  plugins: [react()],
  resolve: { tsconfigPaths: true },
  test: {
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
