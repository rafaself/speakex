import "@testing-library/jest-dom/vitest";

import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

if (!Element.prototype.animate) {
  Object.defineProperty(Element.prototype, "animate", {
    writable: true,
    value: () => ({
      cancel() {},
      commitStyles() {},
      finished: Promise.resolve(),
      finish() {},
      pause() {},
      play() {},
      reverse() {},
      updatePlaybackRate() {},
      addEventListener() {},
      removeEventListener() {},
      dispatchEvent() {
        return true;
      }
    })
  });
}

afterEach(() => {
  cleanup();
});
