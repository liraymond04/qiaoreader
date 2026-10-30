// Tauri doesn't have a Node.js server to do proper SSR
// so we use adapter-static with a fallback to index.html to put the site in SPA mode
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import { initI18n, preloadLocale } from '$lib/i18n';

export const ssr = false;

initI18n();

export async function load() {
  // Avoid a flash of message keys on first paint.
  await preloadLocale();
  return {};
}
