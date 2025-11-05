const framesPerBuffer = 128;

class AudioStreamWorklet extends AudioWorkletProcessor {
  constructor(options) {
    super();

    const { wasmBuffer, wasmGlue, sampleRate } = options.processorOptions;

    // Run the wasm_bindgen setup code, and return the wasm_bindgen object
    const init_wasm_bindgen = new Function(`
  ${wasmGlue}
  return typeof wasm_bindgen !== 'undefined' ? wasm_bindgen : this;
`);
    const wasm_bindgen = init_wasm_bindgen.call({});

    const module = new WebAssembly.Module(wasmBuffer);
    this.wasm = wasm_bindgen.initSync({ module });

    this.processor = new wasm_bindgen.Processor(sampleRate);

    this.port.onmessage = (e) => {
      const { id, value } = e.data;

      const processor = this.processor;
      if (!processor) return;

      processor.set_parameter(id, value);
    }
  }

  process(inputs, outputs, parameters) {
    const processor = this.processor;
    if (!processor) {
      console.error("Missing processor");
      return false;
    }

    const input = inputs[0];
    const output = outputs[0];

    if (input.length == 0 || output.length < 2) {
      console.error("Missing io (inputs: %i, outputs: %i)", input.length, output.length);
      return false;
    }

    const in_left = input[0];
    const in_right = (input.length == 1) ? input[0] : input[1];

    processor.process(in_left, in_right, output[0], output[1], (message) => {
      this.port.postMessage(message);
    });

    return true;
  }
}

registerProcessor('AudioStreamWorklet', AudioStreamWorklet);

///// The wasm_bindgen initialization logic requires TextDecoder, which isn't available in the
///// AudioWorkletNode's scope. These polyfills allow wasm_bindgen initialization to succeed.  

// TextEncoder/TextDecoder polyfills for utf-8 - an implementation of TextEncoder/TextDecoder APIs
// Written in 2013 by Viktor Mukhachev <vic99999@yandex.ru>
// To the extent possible under law, the author(s) have dedicated all copyright and related and neighboring rights to this software to the public domain worldwide. This software is distributed without any warranty.
// You should have received a copy of the CC0 Public Domain Dedication along with this software. If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.

// Some important notes about the polyfill below:
// Native TextEncoder/TextDecoder implementation is overwritten
// String.prototype.codePointAt polyfill not included, as well as String.fromCodePoint
// TextEncoder.prototype.encode returns a regular array instead of Uint8Array
// No options (fatal of the TextDecoder constructor and stream of the TextDecoder.prototype.decode method) are supported.
// TextDecoder.prototype.decode does not valid byte sequences
// This is a demonstrative implementation not intended to have the best performance

// http://encoding.spec.whatwg.org/#textencoder
(function(window) {
  "use strict";
  function TextEncoder() { }
  TextEncoder.prototype.encode = function(string) {
    var octets = [];
    var length = string.length;
    var i = 0;
    while (i < length) {
      var codePoint = string.codePointAt(i);
      var c = 0;
      var bits = 0;
      if (codePoint <= 0x0000007f) {
        c = 0;
        bits = 0x00;
      } else if (codePoint <= 0x000007ff) {
        c = 6;
        bits = 0xc0;
      } else if (codePoint <= 0x0000ffff) {
        c = 12;
        bits = 0xe0;
      } else if (codePoint <= 0x001fffff) {
        c = 18;
        bits = 0xf0;
      }
      octets.push(bits | (codePoint >> c));
      c -= 6;
      while (c >= 0) {
        octets.push(0x80 | ((codePoint >> c) & 0x3f));
        c -= 6;
      }
      i += codePoint >= 0x10000 ? 2 : 1;
    }
    return octets;
  };
  globalThis.TextEncoder = TextEncoder;
  if (!window["TextEncoder"]) window["TextEncoder"] = TextEncoder;

  function TextDecoder() { }
  TextDecoder.prototype.decode = function(octets) {
    if (!octets) return "";
    var string = "";
    var i = 0;
    while (i < octets.length) {
      var octet = octets[i];
      var bytesNeeded = 0;
      var codePoint = 0;
      if (octet <= 0x7f) {
        bytesNeeded = 0;
        codePoint = octet & 0xff;
      } else if (octet <= 0xdf) {
        bytesNeeded = 1;
        codePoint = octet & 0x1f;
      } else if (octet <= 0xef) {
        bytesNeeded = 2;
        codePoint = octet & 0x0f;
      } else if (octet <= 0xf4) {
        bytesNeeded = 3;
        codePoint = octet & 0x07;
      }
      if (octets.length - i - bytesNeeded > 0) {
        var k = 0;
        while (k < bytesNeeded) {
          octet = octets[i + k + 1];
          codePoint = (codePoint << 6) | (octet & 0x3f);
          k += 1;
        }
      } else {
        codePoint = 0xfffd;
        bytesNeeded = octets.length - i;
      }
      string += String.fromCodePoint(codePoint);
      i += bytesNeeded + 1;
    }
    return string;
  };
  globalThis.TextDecoder = TextDecoder;
  if (!window["TextDecoder"]) window["TextDecoder"] = TextDecoder;
})(
  typeof globalThis == "" + void 0
    ? typeof global == "" + void 0
      ? typeof self == "" + void 0
        ? this
        : self
      : global
    : globalThis
);

