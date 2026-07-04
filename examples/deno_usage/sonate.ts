import {
  createEventController,
  type SonateClickEvent,
  type SonateRunHandlers,
  type SonateScrollEvent,
  type SonateEvent,
} from "./events.ts";

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
  sonate_destroy_node: {
    parameters: ["usize", "u64"],
    result: "void",
  },
  sonate_set_parent: {
    parameters: ["usize", "u64", "u64"],
    result: "void",
  },
  sonate_set_attribute: {
    parameters: ["usize", "u64", "buffer", "buffer"],
    result: "void",
  },
  sonate_set_text: {
    parameters: ["usize", "u64", "buffer"],
    result: "void",
  },
  sonate_root_id: {
    parameters: ["usize"],
    result: "u64",
  },
  sonate_set_event_callback: {
    parameters: ["usize", "pointer", "pointer"],
    result: "i32",
  },
  sonate_run: {
    parameters: ["usize"],
    result: "i32",
  },
  // Same native symbol as sonate_run, but executed on Deno's blocking-task
  // thread pool so the JS event loop stays responsive while the window is
  // open. Only valid for engines created with sonate_init(false) (worker
  // mode); in-process mode requires the windowing loop to own the calling
  // (main) thread.
  sonate_run_async: {
    name: "sonate_run",
    parameters: ["usize"],
    result: "i32",
    nonblocking: true,
  },
  sonate_destroy: {
    parameters: ["usize"],
    result: "i32",
  },
});

export const sonate = lib.symbols;

export type {
  SonateClickEvent,
  SonateEvent,
  SonateRunHandlers,
  SonateScrollEvent,
};

const events = createEventController((engine, callbackPtr) =>
  sonate.sonate_set_event_callback(engine, callbackPtr, null),
);

const encoder = new TextEncoder();

export function encode(str: string): ArrayBuffer {
  return encoder.encode(str + "\0").buffer;
}

export const clearEventHandler = events.clearEventHandler;
export const setEventHandler = events.setEventHandler;
export const setClickHandler = events.setClickHandler;

/**
 * Run the engine event loop, blocking the JS thread until the window closes.
 *
 * Only usable with engines created via sonate_init(true). Event handlers run
 * through synchronous same-thread re-entry; timers and other async work do
 * not progress while this call is on the stack.
 */
export function sonate_run(
  engine: bigint,
  handlers: SonateRunHandlers = {},
): number {
  return events.run(
    engine,
    (currentEngine) => sonate.sonate_run(currentEngine),
    handlers,
  );
}

/**
 * Run the engine event loop without blocking the JS event loop.
 *
 * Requires an engine created via sonate_init(false) (worker mode): the
 * window lives in a separate process and events are delivered through a
 * thread-safe callback. Timers, promises and other async work keep running,
 * so state updates from async sources work while the window is open.
 *
 * Resolves with the engine exit code once the window is closed.
 */
export function sonate_run_async(
  engine: bigint,
  handlers: SonateRunHandlers = {},
): Promise<number> {
  return events.runAsync(
    engine,
    (currentEngine) => sonate.sonate_run_async(currentEngine),
    handlers,
  );
}

let nextId = 1n;

export type SonateProps = {
  onclick?: (event: SonateClickEvent) => void;
  [key: string]: unknown;
};

/** A function component. Children are passed via props.children. */
export type ComponentFn = (
  props: SonateProps & { children: SonateNode[] },
) => VNode;

type VChild = VNode | string | number | boolean | null | undefined | VChild[];

/** Normalized child: either an element or text. */
export type SonateNode = VNode | string;

/**
 * A lightweight virtual DOM node. jsx() is pure: it only builds this
 * description. All native engine calls happen in the render/reconcile phase.
 */
export interface VNode {
  type: string | ComponentFn;
  props: SonateProps | null;
  children: SonateNode[];
}

export function jsx(
  type: string | ComponentFn,
  props: SonateProps | null,
  ...children: VChild[]
): VNode {
  return { type, props, children: normalizeChildren(children) };
}

export const jsxs = jsx;

function normalizeChildren(children: VChild[]): SonateNode[] {
  const normalized: SonateNode[] = [];

  const visit = (child: VChild) => {
    if (child === null || child === undefined || typeof child === "boolean") {
      return;
    }
    if (Array.isArray(child)) {
      child.forEach(visit);
      return;
    }
    if (typeof child === "number") {
      normalized.push(String(child));
      return;
    }
    normalized.push(child);
  };

  children.forEach(visit);
  return normalized;
}

// ---------------------------------------------------------------------------
// Instance tree
//
// A persistent tree mirroring what is currently mounted in the engine. It is
// the source of truth for reconciliation: it knows which native node ids are
// alive (so rerenders can clean them up) and stores hook state per component
// instance.
// ---------------------------------------------------------------------------

interface HostInstance {
  kind: "host";
  type: string;
  nodeId: bigint;
  props: SonateProps | null;
  text: string | null;
  /** Mounted element children (text children are folded into `text`). */
  children: Instance[];
}

interface ComponentInstance {
  kind: "component";
  type: ComponentFn;
  props: SonateProps | null;
  children: SonateNode[];
  hooks: HookSlot[];
  rendered: Instance;
  unmounted: boolean;
}

type Instance = HostInstance | ComponentInstance;

interface HookSlot {
  state: unknown;
}

interface RootContext {
  engine: bigint;
  component: ComponentFn;
  instance: ComponentInstance | null;
}

let rootContext: RootContext | null = null;

// Hook bookkeeping for the component currently being rendered.
let currentComponent: ComponentInstance | null = null;
let hookIndex = 0;
let isRendering = false;

/**
 * React-like state hook.
 *
 * State is stored on the component instance in call order, so the usual
 * rules of hooks apply: call useState unconditionally at the top level of
 * the component. The setter triggers a synchronous rerender that reconciles
 * the instance tree against the new output and patches the native document.
 */
export function useState<T>(
  initialState: T | (() => T),
): [T, (next: T | ((previous: T) => T)) => void] {
  const component = currentComponent;
  if (component === null) {
    throw new Error("useState can only be called while rendering a component");
  }

  const index = hookIndex++;
  let slot = component.hooks[index];
  if (slot === undefined) {
    slot = {
      state:
        typeof initialState === "function"
          ? (initialState as () => T)()
          : initialState,
    };
    component.hooks[index] = slot;
  }

  const setState = (next: T | ((previous: T) => T)) => {
    if (component.unmounted) {
      return;
    }
    if (isRendering) {
      throw new Error("Cannot call a state setter while rendering");
    }

    const value =
      typeof next === "function"
        ? (next as (previous: T) => T)(slot.state as T)
        : next;

    if (Object.is(slot.state, value)) {
      return;
    }

    slot.state = value;
    rerender();
  };

  return [slot.state as T, setState];
}

/** Mount a component as the child of the engine's root node. */
export function render(engine: bigint, rootComponent: ComponentFn) {
  if (rootContext !== null && rootContext.instance !== null) {
    unmount(rootContext.instance, true);
  }

  rootContext = { engine, component: rootComponent, instance: null };

  const vnode: VNode = { type: rootComponent, props: null, children: [] };
  const rootId = sonate.sonate_root_id(engine);

  isRendering = true;
  try {
    rootContext.instance = mount(vnode, rootId) as ComponentInstance;
  } finally {
    isRendering = false;
  }
}

function rerender() {
  const context = rootContext;
  if (context === null || context.instance === null) {
    return;
  }

  const vnode: VNode = { type: context.component, props: null, children: [] };
  const rootId = sonate.sonate_root_id(context.engine);

  isRendering = true;
  try {
    const result = reconcile(context.instance, vnode, rootId, true);
    // The root is the sole child of the engine root, so reconcile can always
    // replace it in place and never bails out.
    context.instance = (result ?? mount(vnode, rootId)) as ComponentInstance;
  } finally {
    isRendering = false;
  }
}

function requireRootContext(): RootContext {
  if (rootContext === null) {
    throw new Error("render() must be called before rendering elements");
  }
  return rootContext;
}

function invokeComponent(instance: ComponentInstance): VNode {
  const previousComponent = currentComponent;
  const previousHookIndex = hookIndex;
  currentComponent = instance;
  hookIndex = 0;

  try {
    const output = instance.type({
      ...(instance.props ?? {}),
      children: instance.children,
    });
    if (output === null || typeof output !== "object" || !("type" in output)) {
      throw new Error("Components must return a single element");
    }
    return output;
  } finally {
    currentComponent = previousComponent;
    hookIndex = previousHookIndex;
  }
}

function textContentOf(children: SonateNode[]): string | null {
  let text: string | null = null;
  for (const child of children) {
    if (typeof child === "string") {
      text = (text ?? "") + child;
    }
  }
  return text;
}

function elementChildrenOf(children: SonateNode[]): VNode[] {
  return children.filter((child): child is VNode => typeof child !== "string");
}

function mount(vnode: VNode, parentNodeId: bigint): Instance {
  if (typeof vnode.type === "function") {
    return mountComponent(vnode, parentNodeId);
  }
  return mountHost(vnode, parentNodeId);
}

function mountComponent(vnode: VNode, parentNodeId: bigint): ComponentInstance {
  const instance: ComponentInstance = {
    kind: "component",
    type: vnode.type as ComponentFn,
    props: vnode.props,
    children: vnode.children,
    hooks: [],
    rendered: undefined as unknown as Instance,
    unmounted: false,
  };

  const output = invokeComponent(instance);
  instance.rendered = mount(output, parentNodeId);
  return instance;
}

function mountHost(vnode: VNode, parentNodeId: bigint): HostInstance {
  const { engine } = requireRootContext();
  const nodeId = nextId++;

  const text = textContentOf(vnode.children);
  const textBytes = text === null ? null : encoder.encode(text + "\0");
  const textPtr = textBytes === null ? null : Deno.UnsafePointer.of(textBytes);
  sonate.sonate_create_node(engine, nodeId, textPtr);

  applyProps(engine, nodeId, null, vnode.props);
  sonate.sonate_set_parent(engine, parentNodeId, nodeId);

  const children: Instance[] = [];
  for (const child of elementChildrenOf(vnode.children)) {
    children.push(mount(child, nodeId));
  }

  return {
    kind: "host",
    type: vnode.type as string,
    nodeId,
    props: vnode.props,
    text,
    children,
  };
}

function applyProps(
  engine: bigint,
  nodeId: bigint,
  oldProps: SonateProps | null,
  newProps: SonateProps | null,
) {
  const oldClick = oldProps?.onclick;
  const newClick = newProps?.onclick;
  if (typeof newClick === "function") {
    if (newClick !== oldClick) {
      events.registerNodeClickHandler(engine, nodeId, newClick);
    }
  } else if (typeof oldClick === "function") {
    events.unregisterNodeClickHandler(engine, nodeId);
  }

  if (newProps) {
    for (const [key, value] of Object.entries(newProps)) {
      if (key === "onclick" || typeof value !== "string") {
        continue;
      }
      if (oldProps?.[key] !== value) {
        sonate.sonate_set_attribute(engine, nodeId, encode(key), encode(value));
      }
    }
  }

  if (oldProps) {
    for (const [key, value] of Object.entries(oldProps)) {
      if (key === "onclick" || typeof value !== "string") {
        continue;
      }
      if (newProps === null || !(key in newProps)) {
        // The engine has no attribute removal; overwrite with an empty value.
        sonate.sonate_set_attribute(engine, nodeId, encode(key), encode(""));
      }
    }
  }
}

/**
 * Tear down an instance subtree.
 *
 * Native node destruction is recursive on the engine side, so only the
 * top-most host node of the removed subtree issues a destroy call; the rest
 * of the walk is JS bookkeeping (click handlers, hook invalidation).
 */
function unmount(instance: Instance, destroyNativeSubtree: boolean) {
  if (instance.kind === "component") {
    instance.unmounted = true;
    unmount(instance.rendered, destroyNativeSubtree);
    return;
  }

  const { engine } = requireRootContext();

  if (typeof instance.props?.onclick === "function") {
    events.unregisterNodeClickHandler(engine, instance.nodeId);
  }

  if (destroyNativeSubtree) {
    sonate.sonate_destroy_node(engine, instance.nodeId);
  }

  for (const child of instance.children) {
    unmount(child, false);
  }
}

/**
 * Update `instance` to match `vnode`.
 *
 * Returns the (possibly replaced) instance, or null when the change cannot
 * be patched in place. The engine can only append children (there is no
 * insert-before), so a type change at a position with siblings is handled by
 * the parent host rebuilding its entire child list. `soleChild` marks
 * positions where an in-place replacement is safe (append order preserved).
 */
function reconcile(
  instance: Instance,
  vnode: VNode,
  parentNodeId: bigint,
  soleChild: boolean,
): Instance | null {
  if (instance.kind === "component") {
    if (typeof vnode.type !== "function" || vnode.type !== instance.type) {
      return replaceIfSoleChild(instance, vnode, parentNodeId, soleChild);
    }

    instance.props = vnode.props;
    instance.children = vnode.children;

    const output = invokeComponent(instance);
    const rendered = reconcile(
      instance.rendered,
      output,
      parentNodeId,
      soleChild,
    );
    if (rendered === null) {
      return null;
    }
    instance.rendered = rendered;
    return instance;
  }

  if (typeof vnode.type !== "string" || vnode.type !== instance.type) {
    return replaceIfSoleChild(instance, vnode, parentNodeId, soleChild);
  }

  const { engine } = requireRootContext();

  applyProps(engine, instance.nodeId, instance.props, vnode.props);
  instance.props = vnode.props;

  const text = textContentOf(vnode.children);
  if (text !== instance.text) {
    sonate.sonate_set_text(
      engine,
      instance.nodeId,
      text === null ? null : encode(text),
    );
    instance.text = text;
  }

  reconcileChildren(instance, elementChildrenOf(vnode.children));
  return instance;
}

function replaceIfSoleChild(
  instance: Instance,
  vnode: VNode,
  parentNodeId: bigint,
  soleChild: boolean,
): Instance | null {
  if (!soleChild) {
    return null;
  }
  unmount(instance, true);
  return mount(vnode, parentNodeId);
}

function reconcileChildren(host: HostInstance, newChildren: VNode[]) {
  const oldChildren = host.children;

  const sameShape =
    oldChildren.length === newChildren.length &&
    newChildren.every((vnode, i) => oldChildren[i].type === vnode.type);

  if (sameShape) {
    const soleChild = newChildren.length === 1;
    const reconciled: Instance[] = [];
    let bailedOut = false;

    for (let i = 0; i < newChildren.length; i++) {
      const result = reconcile(
        oldChildren[i],
        newChildren[i],
        host.nodeId,
        soleChild,
      );
      if (result === null) {
        bailedOut = true;
        break;
      }
      reconciled.push(result);
    }

    if (!bailedOut) {
      host.children = reconciled;
      return;
    }
  }

  // The child list changed shape (or a nested change could not be patched in
  // place). Since the engine appends children in creation order, rebuild the
  // whole list to keep sibling order correct.
  for (const oldChild of oldChildren) {
    unmount(oldChild, true);
  }
  host.children = newChildren.map((child) => mount(child, host.nodeId));
}

declare global {
  namespace JSX {
    type Element = VNode;
    interface IntrinsicElements {
      [tag: string]: SonateProps;
    }
  }
}
