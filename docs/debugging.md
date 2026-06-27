# Debugging

Use this to dump style and tree state while the app is running.

- Set `SONATE_DEBUG=1` or `SONATE_DEBUG=true`.
- Press `F9`.
- Two files are written to the current working directory:
  - `style_yyyy-mm-dd_hh-mm-ss_ms.txt`
  - `nodes_yyyy-mm-dd_hh-mm-ss_ms.txt`

If `SONATE_DEBUG` is not set to `1` or `true`, `F9` does nothing.

## Bounds Overlay

Use this to render element bounds on top of the UI.

- Set `SONATE_DEBUG_BOUNDS=1` or `SONATE_DEBUG_BOUNDS=true`.
- Run your app normally.
- Red debug rectangles are drawn for each rendered element bounds.

## Example Commands (Linux/MacOS)

- Dump debug files with F9 enabled:
  - `SONATE_DEBUG=1 cargo run -p sonate --example showcase_input`
- Draw bounds overlay:
  - `SONATE_DEBUG_BOUNDS=1 cargo run -p sonate --example showcase_input`
- Enable both:
  - `SONATE_DEBUG=1 SONATE_DEBUG_BOUNDS=1 cargo run -p sonate --example showcase_input`

## Example Commands (Windows with PowerShell)

- Dump debug files with F9 enabled:
  - `$env:SONATE_DEBUG='1'; cargo run -p sonate --example showcase_input`
- Draw bounds overlay:
  - `$env:SONATE_DEBUG_BOUNDS='1'; cargo run -p sonate --example showcase_input`
- Enable both:
  - `$env:SONATE_DEBUG='1'; $env:SONATE_DEBUG_BOUNDS='1'; cargo run -p sonate --example showcase_input`
- Optional cleanup after running:
  - `Remove-Item Env:\SONATE_DEBUG -ErrorAction SilentlyContinue; Remove-Item Env:\SONATE_DEBUG_BOUNDS -ErrorAction SilentlyContinue`