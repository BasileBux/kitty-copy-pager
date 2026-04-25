# Kitty scrollback copy mode

I love [tmux](https://github.com/tmux/tmux)'s copy mode but I don't want to use it
as it is way too overkill for what I do so I rewrote something similar.

This was made as a scollback pager for the [kitty](https://sw.kovidgoyal.net/kitty/)
terminal. It receives the full scrollback in `stdin` and then renders it and allows
to move with vim motions and copy text.

## NOTES:

The clipboard copy uses [crossterm::clipboard](https://docs.rs/crossterm/latest/crossterm/clipboard/struct.CopyToClipboard.html)
which might not work for 100% of setups but if your terminal doesn't suck it should
work.

This is a work in progress. It should be working for the most part but still be a
bit rough in some places. Super basic vim motions are implemented but more  will
be added in the near future.

```conf
scrollback_pager kitty-copy-mode --flags
```
The scrollback is given in `stdin` to the pager.

This won't work nicely in tmux or software which draws a whole lot of things in
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
- [ ] Even more advanced movement keys
- [ ] Add search functionality (`/` and `?`)
- [ ] More advanced selection motions (`vi"`, `va(`, etc...)
- [ ] Nice UI to indicate copy mode is active

**Missing key Vim movements:**
- `+` / `-` next/prev line (first non-blank)
- `H` / `M` / `L` top/middle/bottom of screen
- `f` / `F` / `t` / `T` find char fwd/back
- `{` / `}` paragraph
- `(` / `)` sentence
- `Ctrl-f` / `Ctrl-b` page down/up
- `%` matching bracket
