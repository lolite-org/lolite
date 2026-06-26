import { sonate, encode, render, jsx } from "sonate";

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
  </div>
);

render(engine, App);
sonate.sonate_run(engine);
sonate.sonate_destroy(engine);
