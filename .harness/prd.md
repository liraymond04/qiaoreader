# Product Requirements Document: QiaoReader
**Target Audience:** Heritage Chinese speakers (Strong auditory comprehension, weak character recognition).
**Core Architecture:** Tauri 2.0 (Rust Core Engine) + Svelte 5/TypeScript (Frontend).
**Package Manager:** Bun.

## Core UX Guardrails
1. **The Auditory Bridge:** Reading is augmented by a sequential companion sidebar displaying target vocabulary in chronological order.
2. **Tap-To-Reveal:** Sidebar cards display only the character by default. Pinyin and English definition layers are CSS-blurred until tapped.
3. **Enforced Mobile Landscape:** On mobile platforms, the reader workspace forces a strict horizontal split-screen orientation (75% eBook content canvas, 25% scrollable reference panel).
4. **Anki Integration:** Direct communication with local `AnkiConnect` JSON-RPC API (`localhost:8765`) bypassing browser CORS walls via Tauri's native Rust execution thread.
