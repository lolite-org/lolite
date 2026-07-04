# Deno bindings example

A minimal React-like layer on top of the sonate FFI bindings:

- `sonate.ts` — FFI bindings, a pure `jsx` factory (virtual DOM), a
  reconciler that patches the native document, and a `useState` hook.
- `events.ts` — event callback plumbing (click/scroll), including a
  thread-safe callback path for the non-blocking run.
- `main.tsx` — demo app with a click counter and an interval-driven ticker.

## Running

Build the engine and the worker binary first, then run the example:

```sh
cargo build
cd examples/deno_usage
deno run --unstable-ffi --allow-ffi --allow-env --allow-read main.tsx
```

## Reactivity

`useState` works like in React: state lives on a per-component instance in
the mounted instance tree, keyed by hook call order. A state setter triggers
a rerender that diffs the new component output against the instance tree and
patches the native document (`sonate_set_attribute`, `sonate_set_text`,
`sonate_destroy_node`, node creation).

Reconciler notes:

- Children are matched by position (no `key` support yet). If a child list
  changes shape, the parent rebuilds its entire child list, because the
  engine can only append children.
- Removed attributes are overwritten with an empty string (the engine has no
  attribute removal).

## Blocking vs non-blocking run

- `sonate_run(engine)` (with `sonate_init(true)`): blocks the JS thread;
  event handlers run via synchronous same-thread re-entry. Timers and other
  async work do not progress while the window is open.
- `await sonate_run_async(engine)` (with `sonate_init(false)`): the window
  runs in a separate worker process and the blocking native call is moved to
  Deno's FFI thread pool, so the JS event loop stays free. Events arrive via
  a thread-safe callback. This enables state updates from async sources
  (e.g. `setInterval`, see the `Ticker` component in `main.tsx`).

The worker binary (`sonate_worker`) is discovered next to the sonate dynamic
library; set `SONATE_WORKER_PATH` to override.
