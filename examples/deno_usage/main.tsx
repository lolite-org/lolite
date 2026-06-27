import {
  sonate,
  encode,
  render,
  jsx,
  sonate_run,
  SonateClickEvent,
} from "sonate";

const engine = sonate.sonate_init(true);

sonate.sonate_add_stylesheet(
  engine,
  encode(`
    .container {
      display: flex;
      flex-direction: row;
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
  `),
);

const App = () => (
  <div class="container">
    <div class="blue-bg">Hello, World!</div>
    <div class="red-bg">Welcome to sonate!</div>
    <button
      type="button"
      class="blue-bg"
      onclick={(event: SonateClickEvent) => {
        console.log("button click handler", {
          x: event.x,
          y: event.y,
          nodeIds: event.nodeIds.map((id) => id.toString()),
        });
      }}
    >
      Click me
    </button>
  </div>
);

render(engine, App);

try {
  sonate_run(engine, {
    onEvent: (event) => {
      if (event.type === "click") {
        console.log("global click", {
          x: event.x,
          y: event.y,
          nodeIds: event.nodeIds.map((id) => id.toString()),
        });
        return;
      } else if (event.type === "scroll") {
        console.log("global scroll", {
          targetId: event.targetId.toString(),
          dx: event.dx,
          dy: event.dy,
        });
      }
    },
  });
} finally {
  sonate.sonate_destroy(engine);
}
