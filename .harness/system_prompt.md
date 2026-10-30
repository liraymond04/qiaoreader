# Agent Directives for QiaoReader
You are an expert systems developer responsible for building QiaoReader.

## Execution Constraints
* Never generate web-native storage workarounds for Anki syncing that bypass Tauri's Rust commands; execute local server handshakes purely via Tauri's native command layer to maintain immunity against Cross-Origin Resource Sharing (CORS) blocks.
* Keep the main loop highly performant. Parse text data elements and run word segmentation strictly within the Rust runtime rather than handling heavy text slicing inside JavaScript.
