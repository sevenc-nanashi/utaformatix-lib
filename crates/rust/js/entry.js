import "./node_modules/fastestsmallesttextencoderdecoder/EncoderDecoderTogether.min.js";
import { File } from "./node_modules/@web-std/file/dist/src/file.cjs";
globalThis.File = File;
globalThis.setTimeout = async (callback, ms) => {
  await sleep(ms);
  callback();
}
export * from "./node_modules/@sevenc-nanashi/utaformatix-ts/base.js";
