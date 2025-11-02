import { execSync } from 'node:child_process';
import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';
import { pluginSvgr } from '@rsbuild/plugin-svgr';
// @ts-expect-error
import TanStackRouterRspack from '@tanstack/router-plugin/rspack';

// Get the current git hash at build time
const getGitHash = () => {
  // If GIT_HASH is provided as an environment variable (e.g., in Docker build), use it
  if (process.env.GIT_HASH) {
    return process.env.GIT_HASH;
  }

  // Otherwise, try to get it from git command (local development)
  try {
    return execSync('git rev-parse --short HEAD').toString().trim();
  } catch (error) {
    console.warn('Could not get git hash:', error);
    return 'unknown';
  }
};

export default defineConfig({
  plugins: [pluginReact(), pluginSvgr()],
  tools: {
    rspack: {
      plugins: [
        TanStackRouterRspack({ target: 'react', autoCodeSplitting: true }),
      ],
    },
  },
  html: {
    title: '',
  },
  source: {
    define: {
      'process.env.GIT_HASH': JSON.stringify(getGitHash()),
      'process.env.API_BASE_URL': JSON.stringify(
        process.env.API_BASE_URL || 'http://localhost:3000',
      ),
    },
  },
});
