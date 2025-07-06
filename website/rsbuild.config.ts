import { defineConfig } from '@rsbuild/core';
import { pluginReact } from '@rsbuild/plugin-react';
import { pluginSvgr } from '@rsbuild/plugin-svgr';
// @ts-ignore
import TanStackRouterRspack from '@tanstack/router-plugin/rspack';

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
});
