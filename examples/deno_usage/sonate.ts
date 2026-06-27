function defaultLibraryName(): string {
  if (Deno.build.os === "windows") {
    return "sonate.dll";
  }

  if (Deno.build.os === "darwin") {
    return "libsonate.dylib";
  }

  return "libsonate.so";
}

function resolveLibraryPath(): string {
  const libraryName = defaultLibraryName();
  const candidates = [
    `../../target/debug/${libraryName}`,
    `../../target/release/${libraryName}`,
  ];

  for (const candidate of candidates) {
    const filePath = decodeURIComponent(
      new URL(candidate, import.meta.url).pathname,
    ).replace(/^\/([A-Za-z]:)/, "$1");

    try {
      Deno.statSync(filePath);
      return filePath;
    } catch {
      // Try the next candidate.
    }
  }

  return decodeURIComponent(
    new URL(candidates[0], import.meta.url).pathname,
  ).replace(/^\/([A-Za-z]:)/, "$1");
}

const libPath = resolveLibraryPath();

const lib = Deno.dlopen(libPath, {
  sonate_init: {
    parameters: ["bool"],
    result: "usize",
  },
  sonate_add_stylesheet: {
    parameters: ["usize", "buffer"],
    result: "void",
  },
  sonate_create_node: {
    parameters: ["usize", "u64", "pointer"],
    result: "u64",
  },
  sonate_set_parent: {
    parameters: ["usize", "u64", "u64"],
    result: "void",
  },
  sonate_set_attribute: {
    parameters: ["usize", "u64", "buffer", "buffer"],
    result: "void",
  },
  sonate_root_id: {
    parameters: ["usize"],
    result: "u64",
  },
  sonate_run: {
    parameters: ["usize"],
    result: "i32",
  },
  sonate_destroy: {
    parameters: ["usize"],
    result: "i32",
  },
});

export const sonate = lib.symbols;

const encoder = new TextEncoder();

export function encode(str: string): ArrayBuffer {
  return encoder.encode(str + "\0").buffer;
}

let nextId = 1n;

type SonateChild = SonateNode | string;

interface SonateNode {
  id: bigint;
  children: SonateNode[];
}

export function jsx(
  _tag: string,
  props: Record<string, string> | null,
  ...children: SonateChild[]
): SonateNode {
  const engine = jsx._engine;
  const id = nextId++;
  const textChildren = children.filter(
    (c): c is string => typeof c === "string",
  );
  const textContent = textChildren.length > 0 ? textChildren.join("") : null;

  const textBytes =
    textContent === null ? null : encoder.encode(textContent + "\0");
  const textPtr = textBytes === null ? null : Deno.UnsafePointer.of(textBytes);
  sonate.sonate_create_node(engine, id, textPtr);

  if (props) {
    for (const [key, value] of Object.entries(props)) {
      if (typeof value === "string") {
        sonate.sonate_set_attribute(engine, id, encode(key), encode(value));
      }
    }
  }

  const nodeChildren: SonateNode[] = [];
  for (const child of children) {
    if (typeof child !== "string") {
      sonate.sonate_set_parent(engine, id, child.id);
      nodeChildren.push(child);
    }
  }

  return { id, children: nodeChildren };
}

export const jsxs = jsx;

jsx._engine = 0n;

export function render(engine: bigint, rootElement: () => SonateNode) {
  jsx._engine = engine;
  const tree = rootElement();
  const rootId = sonate.sonate_root_id(engine);
  sonate.sonate_set_parent(engine, rootId, tree.id);
}

declare global {
  namespace JSX {
    interface IntrinsicElements {
      [tag: string]: Record<string, unknown>;
    }
  }
}
