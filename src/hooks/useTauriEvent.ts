import { useEffect, useRef } from "react";
import { listen, type UnlistenFn } from "@/lib/transport";

export function useTauriEvent<P>(
  eventName: string,
  handler: (payload: P) => void | Promise<void>,
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    let disposed = false;
    let unlisten: UnlistenFn | undefined;

    void (async () => {
      try {
        const off = await listen<P>(eventName, (payload) => {
          void handlerRef.current(payload);
        });
        if (disposed) {
          await off();
        } else {
          unlisten = off;
        }
      } catch (error) {
        console.error(`Failed to subscribe ${eventName} event`, error);
      }
    })();

    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, [eventName]);
}
