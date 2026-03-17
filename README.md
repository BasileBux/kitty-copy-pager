# Kitty scrollback copy mode

## NOTES:

```conf
scrollback_pager kitty-copy-mode --flags
```
The scrollback is given in `stdin` to the pager.

This won't work nicely in tmux or software which draws a whole lot of things in
the terminal. It will remain functional but might not be the best experience.

## TODO:

- [x] Draw scrollback correctly
- [x] Basic movement keys (hjkl, arrow keys) -> scrolling render
- [ ] Initiate selection with `v` and copy with `y` or `enter`, switch end with `o`
- [ ] Copying to clipboard
- [ ] More advanced movement keys (b, w, W, e, 0, $, gg, G, etc...)
- [ ] More advanced selection motions (`vi"`, `va(`, etc...)
- [ ] Nice UI to indicate copy mode is active
- [ ] Window resizing
