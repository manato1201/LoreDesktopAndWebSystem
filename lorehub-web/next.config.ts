import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // Produces a self-contained `.next/standalone` build (a pruned
  // `node_modules` + a `server.js` entrypoint) for the Docker image — see
  // Dockerfile and node_modules/next/dist/docs/01-app/03-api-reference/
  // 05-config/01-next-config-js/output.md. Without this, the runtime image
  // would need the full `node_modules` tree instead of just the files each
  // page actually traces to.
  output: "standalone",
};

export default nextConfig;
