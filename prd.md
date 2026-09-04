Product Requirements Document

Working Title "stash"

One-line description:

Atuin for your clipboard — instant, searchable, context-aware paste history.

Alternative:

A programmable clipboard for developers.

⸻

1. Overview

Build a fast, keyboard-first clipboard history tool written primarily in Rust.

The product runs continuously in the background, captures clipboard history, enriches each entry with useful context, and exposes that history through an fzf/Atuin-inspired popup.

The user invokes the popup with a global shortcut:

⌘⇧V

The popup immediately receives keyboard input.

The user searches, selects an item, presses Enter, and the item is pasted into the application they were previously using.

The fundamental interaction should be:

invoke → search → paste → disappear

The product should feel like part of the operating system rather than an application the user consciously opens.

Unlike traditional clipboard managers, clipboard entries are not merely blobs of text. The system understands:

- what was copied
- when it was copied
- which application it came from
- where possible, which project/repository it came from
- what kind of content it contains
- which actions are relevant to that content

Examples of recognised content include JSON, URLs, JWTs, UUIDs, Git SHAs, shell commands, stack traces, file paths, IP addresses, colours and code.

This allows the product to support contextual search and transformations directly inside the clipboard workflow.

⸻

1. Product Philosophy

The product should follow five principles.

2.1 Clipboard interactions should be transient

The primary UI is not a traditional application window.

It is a popup.

It appears instantly, does one thing, and disappears.

⌘⇧V
type
Enter

The originating application must regain focus immediately.

⸻

2.2 Keyboard first

Every core operation must be possible without touching the mouse.

Mouse support may exist, but it must never be required.

The expected user is comfortable with tools such as:

- fzf
- Atuin
- Vim
- tmux
- Raycast
- command palettes
- terminal applications

⸻

2.3 Context beats organisation

Users should not need to manually organise clipboard history.

Avoid requiring:

- folders
- categories
- manual tags
- collections

Instead, automatically capture context and derive metadata.

The system should understand:

what
when
where
from what application
from what project
what type of data

Search replaces organisation.

⸻

2.4 Understand clipboard contents

Clipboard items should be typed.

A JSON document is different from a URL.

A URL is different from a JWT.

A JWT is different from a stack trace.

Knowing the type allows the product to expose useful actions automatically.

⸻

2.5 Local-first

Clipboard contents are highly sensitive.

The default architecture must therefore be:

- local
- offline
- deterministic
- inspectable

No clipboard data should leave the machine by default.

No account should be required.

No telemetry should contain clipboard contents.

⸻

1. Target User

Primary audience:

Developers and technical power users.

Typical user:

- spends significant time in terminals/editors
- copies commands, URLs, logs and code constantly
- frequently needs something copied minutes or hours ago
- prefers keyboard-driven tools
- uses fuzzy search heavily
- wants less UI, not more UI

Common tools may include:

Ghostty
Terminal
iTerm
VS Code
Android Studio
IntelliJ
Xcode
Neovim
Chrome
Safari
Arc
GitHub
tmux
SSH

⸻

1. Core User Story

A developer copies:

{"user_id":123,"enabled":true,"role":"admin"}

Later they press:

⌘⇧V

The popup appears:

┌───────────────────────────────────────────────────────────────┐
│ > json                                                        │
├───────────────────────────────────────────────────────────────┤
│  2m   {}  {"user_id":123,"enabled":true,...}    Ghostty       │
│  9m   {}  {"name":"foo","version":"1.2"}        Chrome        │
│  1h   {}  {"device":"pixel"}                    Android Stu.   │
├───────────────────────────────────────────────────────────────┤
│ ↑↓ navigate     ↵ paste     ⇥ actions     esc close           │
└───────────────────────────────────────────────────────────────┘

They press:

Enter

The popup disappears.

The JSON is pasted into the original application.

Total interaction:

⌘⇧V
json
↵

⸻

1. Primary UI

The primary interface should resemble an Atuin/fzf search popup.

Example:

┌──────────────────────────────────────────────────────────────────┐
│ > grpc robot                                                     │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  12s  $   grpcurl localhost:50505/...             Ghostty        │
│   3m  📝  UNAVAILABLE: io exception               Android Studio │
│   8m  {}  {"grpc_port":50505,...}                  Ghostty        │
│  23m  🔗  [https://grpc.io/docs/](https://grpc.io/docs/)...                 Arc            │
│   2h  $   adb reverse tcp:50505 tcp:50505          Ghostty        │
│                                                                  │
├──────────────────────────────────────────────────────────────────┤
│ ↑↓ navigate   ↵ paste   ⇥ actions   ⌘e edit   ⌘p pin   esc close │
└──────────────────────────────────────────────────────────────────┘

Characteristics:

- centred floating window
- no title bar
- no Dock activation
- no sidebar
- no permanent chrome
- immediately focused search box
- results update while typing
- keyboard navigation
- optional preview
- fast open/close animation
- should visually resemble a polished terminal tool

The popup should be approximately:

700–900px wide
400–600px high

but configurable.

⸻

1. Default Keyboard Interaction

Invocation

Default:

⌘⇧V

Configurable.

⸻

Navigation

↑ / Ctrl-K       previous result
↓ / Ctrl-J       next result
Enter            paste selected item
Tab              actions
Esc              close
Cmd-E            edit before paste
Cmd-P            pin/unpin
Cmd-D            delete
Cmd-C            copy without closing

Optional Vim mode:

j
k
gg
G

⸻

1. Clipboard Capture

A background process monitors clipboard changes.

Initial supported clipboard types:

MVP

- plain text
- rich text converted to text
- URLs
- file paths

V1

- images
- files
- HTML
- RTF

Every clipboard event creates or updates a history entry.

⸻

1. Clipboard Entry Model

Conceptually:

struct ClipboardEntry {
    id: EntryId,
    created_at: DateTime,
    last_copied_at: DateTime,
    content: ClipboardContent,
    content_hash: String,
    detected_types: Vec,
    source: SourceContext,
    metadata: Metadata,
    copy_count: u32,
    pinned: bool,
}

Source context:

struct SourceContext {
    app_name: Option,
    bundle_id: Option,
    pid: Option,
    cwd: Option,
    git_repo: Option,
    git_root: Option,
    git_branch: Option,
    hostname: Option,
}

Not all fields will always be available.

Missing context must never prevent clipboard capture.

⸻

1. Content Type Detection

The system should automatically classify entries.

Potential types:

PlainText
URL
Email
JSON
YAML
XML
JWT
UUID
IPv4
IPv6
GitSHA
GitHubURL
FilePath
ShellCommand
SQL
StackTrace
SourceCode
Base64
Hex
UnixTimestamp
Colour
Markdown
Image

One item may have multiple classifications.

Example:

[https://github.com/foo/bar/commit/a3c87f](https://github.com/foo/bar/commit/a3c87f)...

could be:

URL
GitHubURL
GitCommitURL

Detection should be:

- fast
- local
- deterministic
- extensible

Detection occurs asynchronously after capture if necessary.

Clipboard capture itself must never wait on expensive detection.

⸻

1. Context-Aware Actions

Actions depend on detected content types.

Press:

Tab

to enter the action palette.

JSON

Paste
Pretty print
Minify
Escape
Unescape
Copy JSONPath...
Validate

URL

Paste
Open
Copy
Remove tracking parameters
Copy domain
Copy path
Convert to curl

JWT

Paste
Decode
Show claims
Show expiry
Copy payload
Copy header

Git SHA

Paste
Copy full SHA
Copy short SHA
git show
Open commit

File path

Paste
Open
Reveal in Finder
Copy basename
Copy parent directory

Base64

Paste
Decode
Copy decoded value

Shell command

Paste
Edit before paste
Copy

Stack trace

Paste
Remove ANSI
Copy exception
Copy first application frame

⸻

1. Transformations

Transformations should be a first-class abstraction.

Conceptually:

trait Transformation {
    fn name(&self) -> &str;
    fn supports(&self, entry: &ClipboardEntry) -> bool;
    fn transform(
        &self,
        entry: &ClipboardEntry
    ) -> Result;
}

Built-in transformations:

Uppercase
Lowercase
Trim
Remove whitespace
JSON pretty
JSON minify
JSON escape
JSON unescape
URL encode
URL decode
Base64 encode
Base64 decode
Hex encode
Hex decode
SHA-256
Remove ANSI
Strip Markdown
Unix timestamp → datetime
Datetime → Unix timestamp
Remove URL tracking parameters

Transformations should compose internally even if transformation pipelines are not initially exposed in the UI.

Future:

Base64 decode
      ↓
JSON pretty
      ↓
paste

⸻

1. Search

Search is one of the core differentiators.

It should feel approximately as fast as fzf.

Typing:

grpc robot

should search:

- clipboard content
- detected type
- source application
- repository
- branch
- directory
- hostname
- metadata

⸻

1. Search Syntax

Normal fuzzy search should require no syntax.

These should simply work:

grpc
robot grpc
android crash
github
jwt
pixel adb

Power-user filters may additionally exist:

app:ghostty
repo:stax
type:json
type:url
host:devbox
branch:main
before:2d
after:1h
is:pinned

Combined:

repo:stax type:json token

⸻

1. Contextual Ranking

Search ranking should incorporate current context.

Example:

The user invokes the popup from:

Ghostty
~/code/stax
feature/foo

Entries copied from the same project should receive a ranking boost.

Conceptual scoring:

text relevance          +0..100
same repository         +30
same application        +20
same hostname           +15
same cwd                +10
same branch              +5
pinned                  +10
recency                  dynamic
frequency                dynamic

Exact weights should be tuned empirically.

The important behaviour:

The clipboard should become more relevant to what the user is currently doing.

⸻

1. Search Modes

Inspired by Atuin.

Potential modes:

Global
Current App
Current Repo
Current Session
Pinned

Users should rarely need to switch modes manually.

Current context should affect ranking automatically.

⸻

1. Deduplication

Repeated copies should not pollute history.

Example:

foo
foo
foo

should become:

foo
copied 3×
last copied 5 sec ago

Entries should primarily deduplicate by content hash.

When duplicated:

copy_count += 1
last_copied_at = now

The item moves back toward the top of history.

⸻

1. Pinning

Users need a lightweight way to retain important values.

Example:

Cmd-P

toggles pinning.

Pinned entries:

- are excluded from automatic retention deletion
- receive a search ranking boost
- can be searched with is:pinned

This replaces traditional clipboard “collections” for most use cases.

⸻

1. Registers

Introduce optional Vim-inspired registers.

Example:

clip register set a

or through the UI.

Registers:

a
b
c
...

A register points to a clipboard entry or contains persistent clipboard content.

CLI:

clip set-register a "hello"
clip get-register a

Potential popup shortcut:

"a

assign selected entry to register a.

Future shortcut:

'a

paste register a.

This feature is optional for MVP but fits the product strongly.

⸻

1. Edit Before Paste

Press:

Cmd-E

The selected entry becomes editable inside the popup.

Example:

┌──────────────────────────────────────────────┐
│                                              │
│ [https://example.com/foo?id=123&utm_source=x](https://example.com/foo?id=123&utm_source=x)  │
│                                              │
├──────────────────────────────────────────────┤
│ ↵ paste edited value             esc cancel  │
└──────────────────────────────────────────────┘

Editing should not mutate the historical entry by default.

The edited value becomes a new clipboard event after paste/copy.

⸻

1. CLI

The product should expose the clipboard database and engine through a CLI.

Binary:

clip

Potential commands:

clip history
clip search grpc
clip latest
clip get 
clip copy "hello"
clip copy < file.json
clip delete 
clip pin 
clip unpin 
clip clear
clip stats

Examples:

clip latest | jq .
git diff | clip copy
clip search jwt
clip search grpc --raw | head -1
clip search   
  --repo stax   
  --type json

Machine-readable output:

clip search grpc --json

should also be supported.

⸻

1. Architecture

Recommended architecture:

```
             OS Clipboard
                  │
                  ▼
          ┌──────────────┐
          │   clipd      │
          │   daemon     │
          └──────┬───────┘
                 │
      ┌──────────┴──────────┐
      │                     │
      ▼                     ▼
```

   ┌─────────────┐       ┌─────────────┐
   │ SQLite DB   │       │ Detection   │
   │ + FTS       │       │ Engine      │
   └─────────────┘       └─────────────┘
          │
          │ IPC
          │
    ┌─────┴───────────────┐
    │                     │
    ▼                     ▼
┌──────────┐        ┌──────────┐
│ clip-ui  │        │   clip   │
│ popup    │        │   CLI    │
└──────────┘        └──────────┘

The daemon owns:

- clipboard monitoring
- persistence
- content detection
- metadata enrichment
- deduplication
- retention
- search
- transformations

Clients communicate through IPC.

Possible IPC:

Unix domain socket

preferred initially.

⸻

1. Rust Workspace

Suggested layout:

clip/
├── Cargo.toml
├── crates/
│
│   ├── clip-core/
│   │   ├── models
│   │   ├── detection
│   │   ├── transformations
│   │   └── search
│   │
│   ├── clip-db/
│   │   ├── sqlite
│   │   └── migrations
│   │
│   ├── clip-platform/
│   │   ├── clipboard
│   │   ├── foreground_app
│   │   └── paste
│   │
│   ├── clip-daemon/
│   │
│   ├── clip-cli/
│   │
│   └── clip-ui/
│
└── migrations/

Keep platform-specific code behind interfaces.

This allows eventual support for:

macOS
Linux
Windows

without contaminating the core.

⸻

1. macOS Platform Integration

macOS is the first-class launch platform.

Required integrations:

Clipboard

Monitor NSPasteboard.

Foreground application

Capture:

application name
bundle ID
PID

Global shortcut

Register configurable system-wide shortcut.

Paste

After selection:

1. place selected/transformed value on system clipboard
2. hide popup
3. restore previous application focus
4. synthesise Cmd-V

This interaction must be extremely reliable.

Popup

Requirements:

- borderless
- floating
- centred
- no Dock icon
- no menu bar activation requirement
- keyboard focus immediately available

Some thin native macOS glue may be necessary even if the majority of the application is Rust.

That is acceptable.

⸻

1. Capturing Terminal Context

Terminal context is particularly valuable but difficult.

Do not make it an MVP blocker.

Phase 1:

Capture:

source app
PID

Phase 2:

Optional shell integration.

Example:

eval "$(clip init zsh)"

The shell integration periodically informs the daemon:

PID
cwd
git root
git branch
hostname
terminal session

Protocol conceptually:

clip context   
    --pid $$   
    --cwd "$PWD"   
    --repo "$(git rev-parse --show-toplevel)"   
    --branch "$(git branch --show-current)"

This is similar philosophically to shell integrations used by modern developer tools.

It should remain optional.

Clipboard functionality must work without it.

⸻

1. SQLite Storage

Use SQLite.

Suggested tables:

entries
sources
types
entry_types
registers
metadata

Simpler MVP schema:

CREATE TABLE entries (
    id TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL,
    last_copied_at INTEGER NOT NULL,
    content_type TEXT NOT NULL,
    text_content TEXT,
    blob_path TEXT,
    content_hash TEXT NOT NULL,
    source_app TEXT,
    source_bundle_id TEXT,
    cwd TEXT,
    git_repo TEXT,
    git_branch TEXT,
    hostname TEXT,
    copy_count INTEGER NOT NULL DEFAULT 1,
    pinned INTEGER NOT NULL DEFAULT 0
);

Indexes:

content_hash
created_at
last_copied_at
source_bundle_id
git_repo
pinned

Use SQLite FTS5 for text search.

⸻

1. Binary Content

Large images/files should not live directly inside SQLite.

Use:

~/.local/share//

or macOS-appropriate application storage.

Example:

data/
├── clipboard.sqlite
└── blobs/
    ├── ab/
    │   └── ab32f...
    └── f9/
        └── f921a...

Content-addressed storage based on hashes allows deduplication.

⸻

1. Retention

Defaults should be conservative.

Example:

history retention       30 days
max text entries        10,000
max image storage       500 MB
pinned retention        forever

All configurable.

Cleanup occurs asynchronously.

⸻

1. Sensitive Data

This requires serious attention.

Potential sensitive types:

passwords
API tokens
JWTs
private keys
credit card numbers
one-time codes

The system should support exclusion rules.

Example:

ignore apps:
    1Password
    Bitwarden
    KeePassXC

Potential behaviour:

Never save copies originating from password managers.

Also support:

ignore applications
ignore content matching regex
ignore clipboard type
pause history

A menu-bar command could expose:

Pause for 5 minutes
Pause for 1 hour
Pause until resumed

⸻

1. Privacy

Default guarantees:

No cloud storage
No account
No clipboard telemetry
No clipboard contents in logs
No clipboard contents in crash reports

If anonymous telemetry is eventually offered, it must be opt-in and limited to non-content events such as:

popup_opened
search_performed
action_invoked

Never:

search query
clipboard contents
file paths
repository names
URLs

⸻

1. Performance Requirements

This product lives or dies on latency.

Targets:

Clipboard capture

< 20 ms perceived impact

Capture should never block the source application.

Popup

Warm:

< 50 ms

from shortcut to visible/searchable UI.

Cold:

< 150 ms

where feasible.

Prefer keeping the lightweight popup process alive if required to achieve this.

Search

For:

10,000 entries

results should update in:

< 30 ms

for common queries.

Paste

From Enter to originating application receiving paste:

< 100 ms

where platform behaviour permits.

⸻

1. Search UX Details

Initial popup with no query:

recent history

Typing immediately filters.

Highlighted matching characters should be visible:

roBOT GRpc

Useful metadata should be dimmer than content.

Result layout:

AGE   TYPE   CONTENT                         SOURCE

Example:

12s    {}    {"foo":"bar"}                   Ghostty
3m     🔗    github.com/foo/bar              Arc
8m     $     adb devices                     Ghostty
1h     📝    IllegalStateException...        Android Studio

Avoid information overload.

Repository/context metadata can appear as a secondary line when useful:

12s  {}  {"foo":"bar"}
         Ghostty · stax · feature/foo

⸻

1. Preview

Preview is useful for large entries.

Potential shortcut:

Cmd-Space

or configurable layout.

Example:

┌──────────────────────────┬─────────────────────────────┐
│ results                  │ preview                     │
│                          │                             │
│ > JSON entry             │ {                           │
│   URL                    │   "foo": "bar",             │
│   command                │   "enabled": true           │
│                          │ }                           │
└──────────────────────────┴─────────────────────────────┘

Preview should not be shown by default if it makes the UI feel heavy.

⸻

1. Configuration

Configuration should be file-based first.

Example:

~/.config//config.toml

Example:

hotkey = "cmd+shift+v"
history_days = 30
max_entries = 10000
vim_keys = true
[ui]
width = 800
height = 500
preview = false
[ignore]
apps = [
    "com.1password.1password",
]
[search]
boost_same_app = 20
boost_same_repo = 30
boost_same_host = 15

A settings UI can come later.

⸻

1. Extensibility

The architecture should anticipate extensions without requiring a plugin system in V1.

Key extensibility points:

detectors
transformations
actions
context providers

Conceptually:

trait Detector {}
trait Transformation {}
trait Action {}
trait ContextProvider {}

Future third-party plugins could potentially be:

WASM
external executables
dynamic libraries

Do not design or ship the plugin system in MVP.

Just avoid architectural decisions that make it impossible.

⸻

1. Future: Custom Actions

Eventually allow users to define shell transformations.

Example:

[[action]]
name = "jq"
command = "jq ."
[[action]]
name = "Remove ANSI"
command = "sed ..."

Then:

clipboard content
       │
       ▼
     stdin
       │
       ▼
 custom command
       │
       ▼
    stdout
       │
       ▼
      paste

This would make the clipboard genuinely programmable.

⸻

1. Future: Transformation Pipelines

Allow named pipelines:

[[pipeline]]
name = "Decode API Response"
steps = [
    "base64_decode",
    "json_pretty"
]

Then:

Tab

> Decode API Response

This should come after basic transformations prove useful.

⸻

1. Future: Sync

Do not build cloud sync initially.

If sync is eventually implemented, prefer:

end-to-end encryption

Potential transports:

user-hosted server
Tailscale
iCloud
Syncthing-style peer-to-peer

Sync should be a separate subsystem.

The core product must remain excellent without it.

⸻

1. Out of Scope for MVP

Explicitly avoid:

- accounts
- cloud storage
- AI features
- team sharing
- screenshot management
- OCR
- elaborate folders
- manual tagging systems
- browser extensions
- mobile applications
- Windows support
- plugin marketplace
- complex preferences UI

These are distractions until the fundamental workflow is excellent.

⸻

1. MVP

The first usable release should contain only enough functionality to validate the central interaction.

MVP requirements

Capture

- monitor text clipboard
- persist history
- deduplicate identical entries
- record timestamp
- record source application

Popup

- global shortcut
- transient popup
- fuzzy search
- keyboard navigation
- paste selected entry
- delete entry
- pin entry
- close with Escape

Detection

Recognise:

Plain text
URL
JSON
JWT
UUID
Git SHA
File path

Actions

Implement:

Paste
Copy
Delete
Pin
Edit before paste

Plus:

JSON pretty
JSON minify
URL open
URL remove tracking parameters
JWT decode

CLI

Implement:

clip history
clip search
clip latest
clip get
clip copy
clip delete
clip pin

Storage

- SQLite
- FTS5
- configurable retention

⸻

1. V1

After MVP proves the workflow:

- shell integration
- repository awareness
- contextual ranking
- images
- rich clipboard formats
- more detectors
- more transformations
- registers
- preview pane
- configurable search modes
- custom actions
- Linux support investigation

⸻

1. Success Criteria

The most important metric is not number of features.

It is:

How quickly can someone recover and paste something they copied previously?

Target common interaction:

⌘⇧V
2–5 characters
Enter

Other qualitative success indicators:

- users replace normal Cmd-V recovery workflows with the tool
- users stop worrying about losing clipboard contents
- search usually finds the intended item in the top 3
- users rarely need the mouse
- popup feels instantaneous
- developers begin using clipboard history as searchable working memory

⸻

1. Key Product Differentiator

Traditional clipboard managers:

copy
  ↓
store
  ↓
browse
  ↓
paste

This product:

```
                ┌─ context
                │
```

copy → understand ──┼─ type
                    │
                    └─ source
          ↓
       search
          ↓
      transform
          ↓
        paste

Or, more simply:

Maccy
  ↓
clipboard history
Buffer
  ↓
rich clipboard history
This product
  ↓
programmable + context-aware clipboard

The important conceptual shift is:

Clipboard entries are structured events, not strings.

⸻

1. The Atuin Analogy

Atuin takes:

shell history

and turns it into:

command

- time
- directory
- session
- host
- context
- powerful search

This product should do the equivalent for:

clipboard history

turning it into:

content

- time
- application
- project
- repository
- host
- type
- transformations
- powerful search

That is the core product thesis.

⸻

1. Ideal Demo

The README should demonstrate the value in approximately ten seconds.

Demo 1 — History

Copy several things.

Press:

⌘⇧V

Search:

grpc

Select an entry.

Press Enter.

It appears in the editor.

Demo 2 — Understanding

Copy:

{"foo":1,"bar":{"hello":"world"}}

Open clipboard.

The item automatically shows:

{}

Press:

Tab

Choose:

Pretty JSON

Press Enter.

The editor receives:

{
  "foo": 1,
  "bar": {
    "hello": "world"
  }
}

Demo 3 — Context

Search:

token

Several matches exist.

The one previously copied from the current repository appears first.

That third demo communicates why this is not just another clipboard manager.

⸻

1. Build Order

Recommended implementation sequence:

01  Clipboard watcher
 ↓
02  SQLite persistence
 ↓
03  CLI history/search
 ↓
04  Popup window
 ↓
05  Search + keyboard navigation
 ↓
06  Reliable paste/focus restoration
 ↓
07  Source application capture
 ↓
08  Content detection
 ↓
09  Action system
 ↓
10  Transformations
 ↓
11  Pins + deduplication
 ↓
12  Shell integration
 ↓
13  Contextual ranking

Do not start by building all the detectors or transformations.

The first milestone should be:

copy foo
copy bar
⌘⇧V

> fo
> Enter
> "foo" appears in the original application

Once that interaction feels ridiculously fast and reliable, build everything else around it.

⸻

1. North Star

The product should eventually feel like the user has acquired a new primitive:

Cmd-C = remember this
Cmd-Shift-V = recall anything

The user should stop thinking:

“What’s currently on my clipboard?”

and instead think:

“I copied that at some point.”

The system finds the rest.