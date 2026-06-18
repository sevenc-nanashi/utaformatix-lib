import { File, Blob } from "@remix-run/web-file";
import { TextEncoder, TextDecoder } from "./encoding.js";

globalThis.File = File;
globalThis.Blob = Blob;
globalThis.TextEncoder = TextEncoder;
globalThis.TextDecoder = TextDecoder;
globalThis.setTimeout = async (callback, ms) => {
  if (ms >= 0) {
    await __host_sleep(ms);
  }
  callback();
};
globalThis.console = {
  log: (...args) => {
    __host_log(0, args.join(" "));
  },
  warn: (...args) => {
    __host_log(1, args.join(" "));
  },
  error: (...args) => {
    __host_log(2, args.join(" "));
  }
};

export * from "./node_modules/@sevenc-nanashi/utaformatix-ts/base.js";
