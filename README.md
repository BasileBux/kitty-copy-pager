# Kitty scrollback copy mode

I love [tmux](https://github.com/tmux/tmux)'s copy mode but I don't want to use it
as it is way too overkill for what I do so I rewrote something similar.

This was made as a scollback pager for the [kitty](https://sw.kovidgoyal.net/kitty/)
terminal. It receives the full scrollback in `stdin` and then renders it and allows
to move with vim motions and copy text.

On [radicle](radicle.xyz): `rad:zDUezH58HjpjpYumsSMjj7dwKLe3`

## Features

Vim motions for moving around, visual mode for selecting text, copying to clipboard.

Mono-lina regex search with highlighting and navigaion with `n` and `N`. There is
real-time search which can be disabled with a CLI flag. Also supports smart case
search which can be disabled with a CLI flag as well.

> [!WARNING] 
> Resizing is not supported. We are working with the raw scrollback which is the
> exact good size but won't work if terminal window is resized.

## NOTES:

The clipboard copy uses [crossterm::clipboard](https://docs.rs/crossterm/latest/crossterm/clipboard/struct.CopyToClipboard.html)
which might not work for 100% of setups but if your terminal doesn't suck it should
work.

This is a work in progress. It should be working for the most part but still be a
bit rough in some places. The performance isn't great for yet but the goal is to
have the smallest startup time. Super basic vim motions are implemented but more
will be added in the near future.

```conf
scrollback_pager kitty-copy-mode --flags
```
The scrollback is given in `stdin` to the pager.

This won't work as nicely in tmux or software which draws a whole lot of things in
the terminal. It will remain functional but might not be the best experience.

## TODO:

- [x] Draw scrollback correctly
- [x] Basic movement keys (hjkl, arrow keys) -> scrolling render
- [x] Initiate selection with `v` and copy with `y` or `enter`, switch end with `o`
- [x] Copying to clipboard
- [x] More advanced movement keys (b, w, W, e, 0, $, gg, G, etc...)
- [x] Rework the movement system
- [x] Try and make the codebase extra clean and extensible
- [x] Handle `\t`
- [x] Handle multi-cell unicode chars
- [x] Add search functionality `/`
    - [x] Add realtime search highlighting (while typing the search query)
    - [x] Add match index in status bar (e.g. `[1/5]`)
- [x] CLI flags for configuration
- [ ] Make initial viewport at the correct position (currently it is always at the bottom)
- [ ] Mouse scroll support (no click but only scroll)
- [ ] Even more advanced movement keys
- [ ] More advanced selection motions (`vi"`, `va(`, etc...)
- [ ] Make the status bar look nicer

**Missing key Vim movements:**
- `+` / `-` next/prev line (first non-blank)
- `H` / `M` / `L` top/middle/bottom of screen
- `f` / `F` / `t` / `T` find char fwd/back
- `{` / `}` paragraph
- `(` / `)` sentence
- `Ctrl-f` / `Ctrl-b` page down/up
- `%` matching bracket
