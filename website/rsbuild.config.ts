import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';
import { pluginSvgr } from '@rsbuild/plugin-svgr';
// @ts-ignore
import TanStackRouterRspack from '@tanstack/router-plugin/rspack';
import { execSync } from 'child_process';

// Get current git hash at build time
const getGitHash = () => {
  try {
    return execSync('git rev-parse HEAD', { encoding: 'utf8' }).trim();
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
    },
  },
});
