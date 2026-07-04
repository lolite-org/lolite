import {
  sonate,
  encode,
  render,
  jsx,
  sonate_run_async,
  useState,
} from "sonate";

// Worker mode (window in a separate process) is required for the
// non-blocking run: it keeps the JS event loop free while the window is
// open, so timers and other async work keep running.
const engine = sonate.sonate_init(false);

sonate.sonate_add_stylesheet(
  engine,
  encode(`
    .container {
      display: flex;
      flex-direction: row;
      gap: 10px;
      padding: 10px;
    }

    .column {
      display: flex;
      flex-direction: column;
      gap: 10px;
      padding: 10px;
    }

    .blue-bg {
      background-color: #7777FF;
      margin: 10px;
      padding: 10px;
    }
    .red-bg {
      background-color: #FF7777;
      margin: 10px;
      padding: 10px;
    }
    .green-bg {
      background-color: #77FF77;
      margin: 10px;
      padding: 10px;
    }
  `),
);

const Counter = () => {
  const [count, setCount] = useState(0);

  return (
    <div class="container">
      <button
        type="button"
        class="blue-bg"
        onclick={() => setCount((current) => current - 1)}
      >
        -
      </button>
      <div class="red-bg">Count: {count}</div>
      <button
        type="button"
        class="blue-bg"
        onclick={() => setCount((current) => current + 1)}
      >
        +
      </button>
      {count % 2 === 0 ? <div class="green-bg">even</div> : null}
    </div>
  );
};

const Ticker = () => {
  const [seconds, setSeconds] = useState(0);

  // The lazy initializer runs exactly once on mount, so it can start the
  // interval without re-registering on every render. Only works because the
  // run loop is non-blocking; unref lets the process exit after the window
  // closes. (A useEffect hook would be the cleaner home for this.)
  useState(() => {
    const timer = setInterval(() => setSeconds((current) => current + 1), 1000);
    Deno.unrefTimer(timer);
    return timer;
  });

  return <div class="red-bg">Uptime: {seconds}s</div>;
};

const App = () => (
  <div class="column">
    <Counter />
    <Ticker />
  </div>
);

render(engine, App);

try {
  const code = await sonate_run_async(engine);
  console.log("window closed with code", code);
} finally {
  sonate.sonate_destroy(engine);
}
