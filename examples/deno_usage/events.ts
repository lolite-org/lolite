const EVENT_CLICK = 1;
const EVENT_SCROLL = 2;

const EVENT_TYPE_OFFSET = 0;
const EVENT_X_OFFSET = 8;
const EVENT_Y_OFFSET = 16;
const EVENT_SCROLL_TARGET_ID_OFFSET = 24;
const EVENT_SCROLL_DX_OFFSET = 32;
const EVENT_SCROLL_DY_OFFSET = 40;
const EVENT_ELEMENT_IDS_PTR_OFFSET = 48;
const EVENT_ELEMENT_COUNT_OFFSET = 56;
const EVENT_STRUCT_SIZE = 64;

export interface SonateClickEvent {
  type: "click";
  x: number;
  y: number;
  nodeIds: bigint[];
}

export interface SonateScrollEvent {
  type: "scroll";
  targetId: bigint;
  dx: number;
  dy: number;
}

export type SonateEvent = SonateClickEvent | SonateScrollEvent;

export interface SonateRunHandlers {
  onEvent?: (event: SonateEvent) => void;
  onClick?: (event: SonateClickEvent) => void;
  onScroll?: (event: SonateScrollEvent) => void;
}

type CallbackRegistration = {
  callback: { close: () => void };
};

type SetEventHandlerOptions = {
  /**
   * Create a thread-safe callback that can be invoked from foreign threads.
   * Required for the non-blocking (worker mode) run, where events arrive on
   * a background listener thread instead of via same-thread re-entry.
   */
  threadSafe?: boolean;
};

type NodeClickHandler = (event: SonateClickEvent) => void;

type SetNativeEventCallback = (
  engine: bigint,
  callbackPtr: Deno.PointerValue,
) => number;

function readNodeIds(pointerValue: bigint, count: number): bigint[] {
  if (pointerValue === 0n || count === 0) {
    return [];
  }

  const idsPtr = Deno.UnsafePointer.create(pointerValue);
  if (idsPtr === null) {
    return [];
  }

  const idsBuffer = new Deno.UnsafePointerView(idsPtr).getArrayBuffer(
    count * 8,
  );
  const idsView = new DataView(idsBuffer);
  const ids: bigint[] = [];
  for (let i = 0; i < count; i++) {
    ids.push(idsView.getBigUint64(i * 8, true));
  }
  return ids;
}

function decodeEvent(eventPtr: Deno.PointerValue): SonateEvent | null {
  if (eventPtr === null) {
    return null;
  }

  const eventBuffer = new Deno.UnsafePointerView(eventPtr).getArrayBuffer(
    EVENT_STRUCT_SIZE,
  );
  const eventView = new DataView(eventBuffer);

  const eventType = eventView.getInt32(EVENT_TYPE_OFFSET, true);
  if (eventType === EVENT_CLICK) {
    const x = eventView.getFloat64(EVENT_X_OFFSET, true);
    const y = eventView.getFloat64(EVENT_Y_OFFSET, true);
    const nodeIdsPtr = eventView.getBigUint64(
      EVENT_ELEMENT_IDS_PTR_OFFSET,
      true,
    );
    const count = Number(
      eventView.getBigUint64(EVENT_ELEMENT_COUNT_OFFSET, true),
    );
    return {
      type: "click",
      x,
      y,
      nodeIds: readNodeIds(nodeIdsPtr, count),
    };
  }

  if (eventType === EVENT_SCROLL) {
    return {
      type: "scroll",
      targetId: eventView.getBigUint64(EVENT_SCROLL_TARGET_ID_OFFSET, true),
      dx: eventView.getFloat64(EVENT_SCROLL_DX_OFFSET, true),
      dy: eventView.getFloat64(EVENT_SCROLL_DY_OFFSET, true),
    };
  }

  return null;
}

export function createEventController(
  setNativeCallback: SetNativeEventCallback,
) {
  const callbackRegistrations = new Map<bigint, CallbackRegistration>();
  const nodeClickHandlers = new Map<bigint, Map<bigint, NodeClickHandler>>();

  function clearEventHandler(engine: bigint) {
    const registration = callbackRegistrations.get(engine);
    if (!registration) {
      return;
    }

    setNativeCallback(engine, null);
    registration.callback.close();
    callbackRegistrations.delete(engine);
  }

  function clearNodeClickHandlers(engine: bigint) {
    nodeClickHandlers.delete(engine);
  }

  function registerNodeClickHandler(
    engine: bigint,
    nodeId: bigint,
    handler: NodeClickHandler,
  ) {
    let handlers = nodeClickHandlers.get(engine);
    if (!handlers) {
      handlers = new Map();
      nodeClickHandlers.set(engine, handlers);
    }
    handlers.set(nodeId, handler);
  }

  function unregisterNodeClickHandler(engine: bigint, nodeId: bigint) {
    nodeClickHandlers.get(engine)?.delete(nodeId);
  }

  function dispatchNodeClickHandlers(engine: bigint, event: SonateClickEvent) {
    const handlers = nodeClickHandlers.get(engine);
    if (!handlers) {
      return;
    }

    for (const nodeId of event.nodeIds) {
      const handler = handlers.get(nodeId);
      if (handler) {
        handler(event);
        return;
      }
    }
  }

  function setEventHandler(
    engine: bigint,
    handler: (event: SonateEvent) => void,
    options: SetEventHandlerOptions = {},
  ) {
    clearEventHandler(engine);

    const definition = {
      parameters: ["usize", "pointer", "pointer"],
      result: "void",
    } as const;

    const onNativeEvent = (
      _handle: number | bigint,
      eventPtr: Deno.PointerValue,
      _userData: Deno.PointerValue,
    ) => {
      const event = decodeEvent(eventPtr);
      if (event !== null) {
        handler(event);
      }
    };

    const callback = options.threadSafe
      ? Deno.UnsafeCallback.threadSafe(definition, onNativeEvent)
      : new Deno.UnsafeCallback(definition, onNativeEvent);

    const code = setNativeCallback(engine, callback.pointer);
    if (code !== 0) {
      callback.close();
      throw new Error("Failed to set sonate event callback");
    }

    callbackRegistrations.set(engine, { callback });
  }

  function setClickHandler(
    engine: bigint,
    handler: (event: SonateClickEvent) => void,
  ) {
    setEventHandler(engine, (event) => {
      if (event.type === "click") {
        handler(event);
      }
    });
  }

  function createDispatcher(engine: bigint, handlers: SonateRunHandlers) {
    return (event: SonateEvent) => {
      handlers.onEvent?.(event);

      if (event.type === "click") {
        dispatchNodeClickHandlers(engine, event);
        handlers.onClick?.(event);
        return;
      }

      handlers.onScroll?.(event);
    };
  }

  function run(
    engine: bigint,
    nativeRun: (engine: bigint) => number,
    handlers: SonateRunHandlers = {},
  ): number {
    setEventHandler(engine, createDispatcher(engine, handlers));

    try {
      return nativeRun(engine);
    } finally {
      clearEventHandler(engine);
    }
  }

  async function runAsync(
    engine: bigint,
    nativeRun: (engine: bigint) => Promise<number>,
    handlers: SonateRunHandlers = {},
  ): Promise<number> {
    setEventHandler(engine, createDispatcher(engine, handlers), {
      threadSafe: true,
    });

    try {
      return await nativeRun(engine);
    } finally {
      clearEventHandler(engine);
    }
  }

  return {
    clearEventHandler,
    clearNodeClickHandlers,
    registerNodeClickHandler,
    unregisterNodeClickHandler,
    run,
    runAsync,
    setClickHandler,
    setEventHandler,
  };
}
