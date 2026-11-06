import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

// vitePreprocess enables <style lang="scss"> in component-scoped style blocks.
export default {
  preprocess: vitePreprocess(),
};
