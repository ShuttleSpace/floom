# floom-editor IDE demo

A miniature workspace, open in a miniature IDE.

The editor on the right is the ported **floom-editor** widget: a rope buffer,
soft wrap, several carets, a gutter, and syntax highlighting supplied by the
demo's own `Styling` implementation.

## What to try

- Click a file in the explorer to open it in a tab.
- Click a tab to switch back. Every file keeps its own caret, selection,
  scroll position and undo history.
- Type. The tab grows a dot while the buffer differs from the file on disk.
- `Cmd/Ctrl + S` writes the file back.
- `Cmd/Ctrl + W` closes the tab.
- `Cmd/Ctrl + 1` … `9` jumps to that tab.
- Hold `Alt` and click to add a caret; type once, and every caret types.
- Drag to select, double-click a word, triple-click a line.

## The files

| Path | What it is |
| --- | --- |
| `src/main.rs` | entry point for a tiny expression language |
| `src/lexer.rs` | the tokenizer |
| `src/parser.rs` | a recursive-descent parser |
| `src/eval.rs` | the evaluator |
| `docs/design.md` | notes on the design |
| `Cargo.toml` | the manifest |

Nothing here is compiled. The workspace exists to give the editor something
worth reading, and something in three different languages to highlight.
