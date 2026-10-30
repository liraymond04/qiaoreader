<script lang="ts">
  import { onMount } from "svelte";
  import type { Snippet } from "svelte";
  import { browser } from "$app/environment";
  import { locale } from "svelte-i18n";

  import { Provider as TooltipProvider } from "$lib/components/ui/tooltip/index.js";

  import "../app.css";

  let { children }: { children: Snippet } = $props();

  onMount(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.classList.toggle("dark", media.matches);
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  });

  $effect(() => {
    if (browser) {
      document.documentElement.lang = $locale ?? "en";
    }
  });
</script>

<TooltipProvider>
  {@render children()}
</TooltipProvider>
