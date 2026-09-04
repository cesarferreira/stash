# stash

Atuin for your clipboard — searchable history with a keyboard-first popup.

## Run

```bash
cargo run
```

Keep that process running. It captures clipboard changes while open (including when hidden).

| Action | How |
|--------|-----|
| Show / hide popup | Global hotkey from config (default **⌘⇧V**) |
| Hide | **Esc** (process keeps running) |
| Quit entirely | **⌘Q** or Dock → Quit |

## Config

On first launch, stash writes:

`~/.config/stash/stash.toml`

```toml
[hotkey]
toggle = "cmd+shift+v"
```

Change `toggle` to a lowercase chord (`cmd+shift+v`, `ctrl+alt+s`, …), then restart stash.

History DB: `~/Library/Application Support/dev.stash.stash/clipboard.sqlite`

## In-popup shortcuts

| Key | Action |
|-----|--------|
| ↑ / Ctrl-K | Previous result |
| ↓ / Ctrl-J | Next result |
| Enter | Copy selected and hide |
| Tab | Action palette |
| Esc | Hide (or back from overlay) |
| ⌘E | Edit before paste |
| ⌘P | Pin / unpin |
| ⌘D | Delete |
| ⌘C | Copy without hiding |

Not yet: synthetic paste into the previous app (still ⌘V after picking), menu-bar icon, login item.
