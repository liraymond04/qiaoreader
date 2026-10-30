<script lang="ts">
  import { t } from "svelte-i18n";
  import XIcon from "@lucide/svelte/icons/x";

  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import {
    Close as DialogClose,
    Content as DialogContent,
    Description as DialogDescription,
    Header as DialogHeader,
    Root as Dialog,
    Title as DialogTitle,
  } from "$lib/components/ui/dialog/index.js";
  import { Separator } from "$lib/components/ui/separator/index.js";
  import type { DictWord } from "$lib/dictionary/types";

  import PinyinText from "./PinyinText.svelte";

  let {
    word,
    toneColors = true,
    onClose,
  }: {
    word: DictWord | null;
    toneColors?: boolean;
    onClose: () => void;
  } = $props();
</script>

<Dialog
  open={word !== null}
  onOpenChange={(open) => {
    if (!open) onClose();
  }}
>
  <DialogContent
    showCloseButton={false}
    class="max-h-[85dvh] w-[calc(100%-2rem)] max-w-lg overflow-hidden"
  >
    {#if word}
      <DialogHeader class="pr-10">
        <DialogTitle class="font-reader text-2xl">{word.text}</DialogTitle>
        <DialogDescription>
          {#if word.entries[0]}
            <PinyinText
              entry={word.entries[0]}
              {toneColors}
              className="text-sm"
            />
          {/if}
        </DialogDescription>
      </DialogHeader>

      <div class="-mx-1 max-h-[65dvh] overflow-y-auto px-1">
        {#each word.entries as entry, index}
          <section class="space-y-2 py-3">
            {#if index > 0}
              <div class="flex flex-wrap items-center gap-2">
                <PinyinText
                  {entry}
                  {toneColors}
                  className="text-sm font-medium"
                />
                {#if entry.traditional !== entry.simplified}
                  <Badge variant="outline">
                    {$t("details.traditional")} · {entry.traditional}
                  </Badge>
                {/if}
              </div>
            {:else if entry.traditional !== entry.simplified}
              <div class="flex flex-wrap items-center gap-2">
                <Badge variant="outline">
                  {$t("details.traditional")} · {entry.traditional}
                </Badge>
              </div>
            {/if}
            <ol class="list-decimal space-y-1 pl-5 text-sm text-muted-foreground">
              {#each entry.senses as sense}
                <li>{sense.definition}</li>
              {/each}
            </ol>
          </section>
          {#if index < word.entries.length - 1}
            <Separator />
          {/if}
        {/each}
      </div>

      <DialogClose>
        {#snippet child({ props })}
          <Button
            {...props}
            variant="ghost"
            size="icon-sm"
            class="absolute right-3 top-3"
            aria-label={$t("common.close")}
          >
            <XIcon />
          </Button>
        {/snippet}
      </DialogClose>
    {/if}
  </DialogContent>
</Dialog>
