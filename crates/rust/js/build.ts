import { rollup } from "rollup";
import commonjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import json from "@rollup/plugin-json";
import polyfill from "rollup-plugin-polyfill-node";
import alias from "@rollup/plugin-alias";

const bundle = await rollup({
  input: `${import.meta.dirname}/entry.js`,
  plugins: [
    commonjs(),
    json(),
    nodeResolve(),
    polyfill(),
    alias({
      entries: [
        {
          find: "web-encoding",
          replacement: `${import.meta.dirname}/encoding.js`,
        },
      ],
    }),
  ],
});

await bundle.write({
  file: `${import.meta.dirname}/../src/utaformatix.js`,
  format: "iife",
  name: "utaformatix",
});
