export class TextEncoder {
  encode(str) {
    return __encode(str);
  }
}
export class TextDecoder {
  encoding = "utf-8";
  constructor(encoding = "utf-8") {
    this.encoding = encoding;
  }
  decode(bytes) {
    return __decode(bytes, this.encoding);
  }
}
