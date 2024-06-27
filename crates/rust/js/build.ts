import { rollup } from "rollup";
import commonjs from "@rollup/plugin-commonjs";
import nodeResolve from "@rollup/plugin-node-resolve";
import json from "@rollup/plugin-json";
import polyfill from "rollup-plugin-polyfill-node";

const bundle = await rollup({
  input: `${import.meta.dirname}/entry.js`,
  plugins: [commonjs(), json(), nodeResolve(), polyfill()],
});

await bundle.write({
  file: `${import.meta.dirname}/../src/utaformatix.js`,
  format: "iife",
  name: "utaformatix",
});
