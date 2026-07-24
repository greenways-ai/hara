# Hara REPL UX specification

The REPL is a JLine-based interactive shell. It provides line editing, persistent history,
completion, and inline documentation while evaluating ordinary Hara forms.

## Session flow

```text
start
  |
  v
custom splash / session banner
  |
  v
read line --edit-- history --complete-- docs
  |
  v
read complete Hara form
  |
  v
evaluate -> print value
  |
  +----> print friendly error and continue
  |
  +----> EOF -> exit
```

The default banner uses large Hara ASCII art: an alien craft beams a clear-to-blue-to-black
gradient onto dimensional `HARA` lettering. It includes the tagline `Journey Within` and a spaced
top menu.
It identifies runtime `TRUFFLE` and session `ROOT`.
The left prompt is namespace-only, for example `[user] `, while the right prompt reports the
live RESP endpoint or `offline`. Embedders may replace
the splash or suppress it through `ReplConfig`; `hara.repl.splash` and `hara.repl.no-color` provide
process-level configuration. Banner customization must not change evaluation semantics.

## History

History is persisted in the user's Hara history file and is incrementally updated while editing:

```text
previous session ---> ~/.hara_history <--- current session
       ^                                      |
       +------ up/down, search, replay -------+
```

History stores entered forms, not evaluated values. Implementations should avoid writing secrets
or incomplete input when the host provides a history filter.

## Completion

Completion has two sources:

```text
typed prefix
     |
     +--> visible Hara symbols ----+
     |                              |
     +--> slash commands ----------+--> candidates
```

Completion appears as a cursor-level ghost suggestion while typing, including immediately after `/`.
Tab accepts and cycles matching slash commands or symbols. Completion understands Lisp delimiters: `(`, `)`, `[`, `]`, `{`, `}`, and whitespace. Symbol
completion comes from the runtime-visible namespace. Host Java classes are not ambient language
completion candidates.

Examples:

```text
(con<TAB>       -> concat
/re<TAB>        -> /resp
```

## Documentation widget

The documentation widget is bound to common help keys, including `Alt-q`, `F1`, and supported
Shift-Enter terminal sequences:

```text
cursor on symbol -> inspect metadata -> show doc/arglists -> redraw prompt
```

Missing metadata is a no-op. Documentation output must not alter the input buffer or evaluation
state.

## Slash-command boundary

Slash commands are intentionally a separate REPL control layer. They are not Hara symbols and are
never sent to the evaluator:

```text
input line
   |
   +--> starts with / ? -- yes --> REPL command dispatcher
   |                                  |
   |                                  +--> /help /history /clear /splash /status /resp /ns /quit
   |
   +--> no ------------------------> Hara reader/evaluator
```

`/resp` reports listener status; `/resp start [PORT|HOST:PORT]`, `/resp stop`, and
`/resp restart [PORT|HOST:PORT]` control the listener without replacing `ROOT`.

The command vocabulary is an extension point. Unknown commands should produce a concise REPL
error and leave the session running. A slash command must not shadow a valid Hara form because the
dispatch decision is made only at the beginning of an input line.
