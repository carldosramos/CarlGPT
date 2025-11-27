import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  experimental: {
    dynamicIO: true,
    // @ts-expect-error - cacheComponents is not yet in the types
    cacheComponents: true,
    cacheLife: {
      default: { stale: 60, revalidate: 60 },
    },
  },
};

export default nextConfig;
