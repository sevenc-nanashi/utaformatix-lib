import { File } from "./node_modules/@web-std/file/dist/src/file.cjs";
import { Blob } from "./node_modules/@web-std/blob/dist/src/blob.cjs";
import { TextEncoder, TextDecoder } from "./encoding.js";

globalThis.File = File;
globalThis.Blob = Blob;
globalThis.TextEncoder = TextEncoder;
globalThis.TextDecoder = TextDecoder;
globalThis.setTimeout = async (callback, ms) => {
  if (ms >= 0) {
    await __sleep(ms);
  }
  callback();
};

export * from "./node_modules/@sevenc-nanashi/utaformatix-ts/base.js";
