import { invoke } from "@tauri-apps/api/core";

import type {
  DictionaryBuildReport,
  DictionarySourceInfo,
  DictionaryStatus,
  ProcessedText,
} from "./types";

export function dictionaryStatus(): Promise<DictionaryStatus> {
  return invoke<DictionaryStatus>("dictionary_status");
}

export function dictionarySource(): Promise<DictionarySourceInfo> {
  return invoke<DictionarySourceInfo>("dictionary_source");
}

export function buildDictionary(force: boolean): Promise<DictionaryBuildReport> {
  return invoke<DictionaryBuildReport>("dictionary_build", { force });
}

export function processRawText(text: string): Promise<ProcessedText> {
  return invoke<ProcessedText>("process_raw_text", { text });
}
