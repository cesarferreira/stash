# stash

Atuin for your clipboard — interactive GPUI prototype.

## Run

```bash
cargo run
```

## Prototype shortcuts

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

This draft uses in-memory PRD sample data only — no clipboard watcher, daemon, or synthetic paste.
