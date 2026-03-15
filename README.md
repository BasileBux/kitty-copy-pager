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
- [ ] Basic movement keys (hjkl, arrow keys) -> scrolling render
- [ ] Initiate selection with `v` and copy with `y` or `esc` or `enter`
- [ ] Copying to clipboard
- [ ] More advanced movement keys (b, w, e, 0, $, gg, G)
- [ ] Nice UI to indicate copy mode is active
- [ ] Window resizing
