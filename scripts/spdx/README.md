# Standard licence texts

`generate-third-party-licenses.mjs` falls back to these when a dependency declares a licence in its
manifest but publishes no text of its own (the `objc2` family, `webview2-com`, `tauri-plugin`,
`whatlang`, `selectors`, `clipboard-win` and `alloc-stdlib` in the current tree).

Each file is the SPDX license list's own text for the identifier in its name, fetched on 2026-11-10
from `https://raw.githubusercontent.com/spdx/license-list-data/main/text/<identifier>.txt` and not
edited. Five of the six are byte-identical to the copies backlog-atlas fetched on 2026-08-14.

`MIT`, `BSD-3-Clause` and `Zlib` are templates and keep SPDX's placeholders (`<year>`,
`<copyright holders>`): a package that publishes no notice names no holder, and the generator labels
every such entry `(standard text; this package publishes none)`.

Add a file here only after looking at the licence a dependency has newly brought in. The generator
stops and names the identifier when it has no text for it; that stop is the review point.
