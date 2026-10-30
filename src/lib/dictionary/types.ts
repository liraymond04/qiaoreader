/** Dictionary payloads from the Rust core. Field names are camelCase. */

export type DictSense = {
  position: number;
  definition: string;
};

export type DictEntry = {
  id: number;
  traditional: string;
  simplified: string;
  pinyin: string;
  pinyinDisplay: string;
  pinyinPlain: string;
  wordLength: number;
  senses: DictSense[];
};

export type DictWord = {
  text: string;
  entries: DictEntry[];
};

export type TextSegment = {
  text: string;
  isWord: boolean;
  /** Index into `ProcessedText.words`, or null when there is no entry. */
  wordIndex: number | null;
};

export type ProcessedText = {
  segments: TextSegment[];
  words: DictWord[];
};

export type DictionaryStatus = {
  ready: boolean;
  dbPath: string;
  dbBytes: number;
  schemaVersion: number;
  entryCount: number;
  senseCount: number;
  sourcePath: string | null;
  sourceSha256: string | null;
  sourceBytes: number | null;
  builtAtUnix: number | null;
  buildMs: number | null;
  skippedLines: number | null;
  cedictVersion: string | null;
  cedictDate: string | null;
  cedictPublisher: string | null;
  cedictLicense: string | null;
};

export type DictionarySourceInfo = {
  found: boolean;
  path: string | null;
  bytes: number | null;
  searched: string[];
};

export type ImportProgress = {
  phase: string;
  processed: number;
  total: number;
  entries: number;
};

export type DictionaryBuildReport = {
  status: DictionaryStatus;
  skipped: boolean;
  entriesImported: number;
  sensesImported: number;
  skippedLines: number;
  warnings: string[];
  elapsedMs: number;
  sourcePath: string;
  sourceSha256: string;
};
