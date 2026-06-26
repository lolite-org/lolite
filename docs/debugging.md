# Debugging

Use this to dump style and tree state while the app is running.

- Set `LOLITE_DEBUG=1` or `LOLITE_DEBUG=true`.
- Press `F9`.
- Two files are written to the current working directory:
  - `style_yyyy-mm-dd_hh-mm-ss_ms.txt`
  - `nodes_yyyy-mm-dd_hh-mm-ss_ms.txt`

If `LOLITE_DEBUG` is not set to `1` or `true`, `F9` does nothing.