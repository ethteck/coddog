/// <reference types="@rsbuild/core/types" />

declare namespace NodeJS {
  interface ProcessEnv {
    GIT_HASH: string;
  }
}

declare module "*.svg" {
  const content: string;
  export default content;
}
