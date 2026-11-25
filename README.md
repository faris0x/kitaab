# kitaab

Read, listen to, and generate subtitles from the Noble Qur'an, in your terminal.

> **Status: in progress — largely non-functional.** This is an early, partially-built
> clone under active development. Most features are incomplete, unpolished, or
> broken. Do not rely on it. Use at your own risk.

## What's here so far

- Fetch-on-first-run Quran data (Arabic, transliteration, English, Turkish) with local caching
- Terminal reader with paged reading, Arabic text shaping, and reverse (RTL) toggle
- Per-surah recitation audio (streaming download + playback) with Ayatul Kursi support
- SRT subtitle generation (`sub` command)
- Bookmark manager (`bookmarks`/`bm` command)
- Settings screen for the colour theme and reading/subtitle options (`settings`/`st`), saved automatically
- Surah list, main menu with fuzzy surah search

## Building

Requires Rust (edition 2024) and ALSA development libraries on Linux.

```sh
cargo build --release
./target/release/kitaab
```

## Usage

Run `kitaab`, then type a surah number or name at the prompt, or one of:
`list`/`ls`, `bookmarks`/`bm`, `sub`, `settings`/`st`, `quit`/`q`. `Ctrl+C` quits.

## License

GPL-3.0-only. See [LICENSE](LICENSE).

## Acknowledgements

Inspired by the [QuranCLI](https://github.com/anonfaded/QuranCLI) project.
Quran data and recitations are fetched from public sources on first run and
cached locally.
