import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const rpc = readFileSync(new URL("../templates/rust-driver/src/rpc.rs", import.meta.url), "utf8");

describe("rust-driver rpc template", () => {
  // The host only receives the error message, not the code, and uses its
  // fallback for optional methods when the message says "method not found"
  // (is_method_not_found in src-tauri/src/plugins/driver.rs).
  it("answers unimplemented methods with a message that triggers the host fallbacks", () => {
    const message = /pub fn not_implemented[\s\S]*?format!\("([^"]*)"/.exec(rpc)?.[1];
    expect(message).toMatch(/method not found/i);
  });
});
