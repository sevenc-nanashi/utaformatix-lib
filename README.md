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

- This library contains a JS runtime. It uses [QuickJS](https://bellard.org/quickjs/) by default, and can use [Boa JS](https://boajs.dev) with the `boa` feature.
  - To use Boa without QuickJS, enable `boa` with `default-features = false`. As Boa JS is pure-rust, you can create pure-rust binaries without any C/C++ dependencies, but it causes significant performance degradation.
- The library embeds [utaformatix-ts](https://github.com/sevenc-nanashi/utaformatix-ts), and uses it to run UtaFormatix code.
  - There are some polyfills for filling the gap between Node.js and the embedded JS runtime. See [crates/rust/js] for details.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
