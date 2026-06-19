# utaformatix-lib / [UtaFormatix](https://github.com/sdercolin/utaformatix3) wrapper for many programming languages

utaformatix-lib is a library that allows you to use UtaFormatix in many programming languages.

## Languages

- Rust
- C (as dll, WIP)
- C++ (WIP)
- Ruby (WIP)
- Python (WIP)

- <s>TypeScript / JavaScript</s> (Never, just use [utaformatix-ts](https://github.com/sevenc-nanashi/utaformatix-ts))

## How does it work?

- The library contains a JS runtime. It uses [Boa JS](https://boajs.dev) by default, and can use QuickJS with the `quickjs` feature.
  - To use QuickJS without Boa, enable `quickjs` with `default-features = false`.
- The library embeds [utaformatix-ts](https://github.com/sevenc-nanashi/utaformatix-ts), and uses it to run UtaFormatix code.
  - There are some polyfills for filling the gap between Node.js and the embedded JS runtime. See [crates/rust/js] for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
