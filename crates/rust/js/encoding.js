export class TextEncoder {
  encode(str) {
    return __host_encode(str);
  }
}
export class TextDecoder {
  encoding = "utf-8";
  constructor(encoding = "utf-8") {
    this.encoding = encoding;
  }
  decode(bytes) {
    return __host_decode(bytes, this.encoding);
  }
}
