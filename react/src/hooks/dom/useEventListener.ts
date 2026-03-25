import { useEffect, useRef } from 'react';

function useEventListener(
  eventName: string,
  handler: (event: Event) => void,
  element: EventTarget = window
) {
  // Create a ref that stores the handler
  const savedHandler = useRef<(event: Event) => void>();

  // Update ref.current value if handler changes.
  // This allows our effect below to always get the latest handler without us needing to pass it in effect deps array
  // and potentially cause unnecessary re-renders.
  useEffect(() => {
    savedHandler.current = handler;
  }, [handler]);

  useEffect(() => {
    // Make sure element supports addEventListener
    // On
    const isSupported = element?.addEventListener;
    if (!isSupported) {
      return;
    }

    // Create event listener that calls handler function stored in ref
    const eventListener = (event: Event) => savedHandler.current?.(event);

    // Add event listener
    element.addEventListener(eventName, eventListener);

    // Remove event listener on cleanup
    return () => {
      element.removeEventListener(eventName, eventListener);
    };
  }, [eventName, element]); // Re-run if eventName or element changes
}

export default useEventListener;
