import { rolldown } from "rolldown";

const bundle = await rolldown({
  input: `${import.meta.dirname}/entry.js`,
  resolve: {
    alias: {
      "web-encoding": `${import.meta.dirname}/encoding.js`,
    },
  },
});

await bundle.write({
  file: `${import.meta.dirname}/../src/utaformatix.js`,
  format: "iife",
  name: "utaformatix",
  minify: true,
});
