# stash

Atuin for your clipboard — searchable history with a keyboard-first popup.

## Run

```bash
cargo run
```

History is stored on disk at:

`~/Library/Application Support/dev.stash.stash/clipboard.sqlite`

(plus image blobs under `blobs/`). It persists across quits and reboots. Clipboard capture runs while the app is open.

## Shortcuts

| Key | Action |
|-----|--------|
| ↑ / Ctrl-K | Previous result |
| ↓ / Ctrl-J | Next result |
| Enter | Copy selected (or run action / paste edited) and quit |
| Tab | Action palette |
| Esc | Close / back |
| ⌘E | Edit before paste |
| ⌘P | Pin / unpin |
| ⌘D | Delete |
| ⌘C | Copy without closing |

Not yet: background daemon, global ⌘⇧V, or synthetic paste into the previous app.
