var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __esm = (fn, res, err) => function __init() {
  if (err) throw err[0];
  try {
    return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
  } catch (e) {
    throw err = [e], e;
  }
};
var __commonJS = (cb, mod) => function __require() {
  try {
    return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
  } catch (e) {
    throw mod = 0, e;
  }
};
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// node_modules/@anthropic-ai/sdk/internal/tslib.mjs
function __classPrivateFieldSet(receiver, state, value, kind, f) {
  if (kind === "m")
    throw new TypeError("Private method is not writable");
  if (kind === "a" && !f)
    throw new TypeError("Private accessor was defined without a setter");
  if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver))
    throw new TypeError("Cannot write private member to an object whose class did not declare it");
  return kind === "a" ? f.call(receiver, value) : f ? f.value = value : state.set(receiver, value), value;
}
function __classPrivateFieldGet(receiver, state, kind, f) {
  if (kind === "a" && !f)
    throw new TypeError("Private accessor was defined without a getter");
  if (typeof state === "function" ? receiver !== state || !f : !state.has(receiver))
    throw new TypeError("Cannot read private member from an object whose class did not declare it");
  return kind === "m" ? f : kind === "a" ? f.call(receiver) : f ? f.value : state.get(receiver);
}
var init_tslib = __esm({
  "node_modules/@anthropic-ai/sdk/internal/tslib.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/uuid.mjs
var uuid4;
var init_uuid = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/uuid.mjs"() {
    uuid4 = function() {
      const { crypto: crypto2 } = globalThis;
      if (crypto2?.randomUUID) {
        uuid4 = crypto2.randomUUID.bind(crypto2);
        return crypto2.randomUUID();
      }
      const u8 = new Uint8Array(1);
      const randomByte = crypto2 ? () => crypto2.getRandomValues(u8)[0] : () => Math.random() * 255 & 255;
      return "10000000-1000-4000-8000-100000000000".replace(/[018]/g, (c) => (+c ^ randomByte() & 15 >> +c / 4).toString(16));
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/errors.mjs
function isAbortError(err) {
  return typeof err === "object" && err !== null && // Spec-compliant fetch implementations
  ("name" in err && err.name === "AbortError" || // Expo fetch
  "message" in err && String(err.message).includes("FetchRequestCanceledException"));
}
var castToError;
var init_errors = __esm({
  "node_modules/@anthropic-ai/sdk/internal/errors.mjs"() {
    castToError = (err) => {
      if (err instanceof Error)
        return err;
      if (typeof err === "object" && err !== null) {
        try {
          if (Object.prototype.toString.call(err) === "[object Error]") {
            const error = new Error(err.message, err.cause ? { cause: err.cause } : {});
            if (err.stack)
              error.stack = err.stack;
            if (err.cause && !error.cause)
              error.cause = err.cause;
            if (err.name)
              error.name = err.name;
            return error;
          }
        } catch {
        }
        try {
          return new Error(JSON.stringify(err));
        } catch {
        }
      }
      return new Error(err);
    };
  }
});

// node_modules/@anthropic-ai/sdk/core/error.mjs
var AnthropicError, APIError, APIUserAbortError, APIConnectionError, APIConnectionTimeoutError, RetryableError, BadRequestError, AuthenticationError, PermissionDeniedError, NotFoundError, ConflictError, UnprocessableEntityError, RateLimitError, InternalServerError;
var init_error = __esm({
  "node_modules/@anthropic-ai/sdk/core/error.mjs"() {
    init_errors();
    AnthropicError = class extends Error {
    };
    APIError = class _APIError extends AnthropicError {
      constructor(status, error, message, headers, type) {
        super(`${_APIError.makeMessage(status, error, message)}`);
        this.status = status;
        this.headers = headers;
        this.requestID = headers?.get("request-id");
        this.error = error;
        this.type = type ?? null;
      }
      static makeMessage(status, error, message) {
        const msg = error?.message ? typeof error.message === "string" ? error.message : JSON.stringify(error.message) : error ? JSON.stringify(error) : message;
        if (status && msg) {
          return `${status} ${msg}`;
        }
        if (status) {
          return `${status} status code (no body)`;
        }
        if (msg) {
          return msg;
        }
        return "(no status code or body)";
      }
      static generate(status, errorResponse, message, headers) {
        if (!status || !headers) {
          return new APIConnectionError({ message, cause: castToError(errorResponse) });
        }
        const error = errorResponse;
        const type = error?.["error"]?.["type"];
        if (status === 400) {
          return new BadRequestError(status, error, message, headers, type);
        }
        if (status === 401) {
          return new AuthenticationError(status, error, message, headers, type);
        }
        if (status === 403) {
          return new PermissionDeniedError(status, error, message, headers, type);
        }
        if (status === 404) {
          return new NotFoundError(status, error, message, headers, type);
        }
        if (status === 409) {
          return new ConflictError(status, error, message, headers, type);
        }
        if (status === 422) {
          return new UnprocessableEntityError(status, error, message, headers, type);
        }
        if (status === 429) {
          return new RateLimitError(status, error, message, headers, type);
        }
        if (status >= 500) {
          return new InternalServerError(status, error, message, headers, type);
        }
        return new _APIError(status, error, message, headers, type);
      }
    };
    APIUserAbortError = class extends APIError {
      constructor({ message } = {}) {
        super(void 0, void 0, message || "Request was aborted.", void 0);
      }
    };
    APIConnectionError = class extends APIError {
      constructor({ message, cause }) {
        super(void 0, void 0, message || "Connection error.", void 0);
        if (cause)
          this.cause = cause;
      }
    };
    APIConnectionTimeoutError = class extends APIConnectionError {
      constructor({ message } = {}) {
        super({ message: message ?? "Request timed out." });
      }
    };
    RetryableError = class extends AnthropicError {
      constructor(message, { cause } = {}) {
        super(message ?? "Retryable error.");
        if (cause !== void 0)
          this.cause = cause;
      }
    };
    BadRequestError = class extends APIError {
    };
    AuthenticationError = class extends APIError {
    };
    PermissionDeniedError = class extends APIError {
    };
    NotFoundError = class extends APIError {
    };
    ConflictError = class extends APIError {
    };
    UnprocessableEntityError = class extends APIError {
    };
    RateLimitError = class extends APIError {
    };
    InternalServerError = class extends APIError {
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/values.mjs
function maybeObj(x) {
  if (typeof x !== "object") {
    return {};
  }
  return x ?? {};
}
function isEmptyObj(obj) {
  if (!obj)
    return true;
  for (const _k in obj)
    return false;
  return true;
}
function hasOwn(obj, key) {
  return Object.prototype.hasOwnProperty.call(obj, key);
}
var startsWithSchemeRegexp, isAbsoluteURL, isArray, isReadonlyArray, validatePositiveInteger, safeJSON;
var init_values = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/values.mjs"() {
    init_error();
    startsWithSchemeRegexp = /^[a-z][a-z0-9+.-]*:/i;
    isAbsoluteURL = (url) => {
      return startsWithSchemeRegexp.test(url);
    };
    isArray = (val) => (isArray = Array.isArray, isArray(val));
    isReadonlyArray = isArray;
    validatePositiveInteger = (name, n) => {
      if (typeof n !== "number" || !Number.isInteger(n)) {
        throw new AnthropicError(`${name} must be an integer`);
      }
      if (n < 0) {
        throw new AnthropicError(`${name} must be a positive integer`);
      }
      return n;
    };
    safeJSON = (text) => {
      try {
        return JSON.parse(text);
      } catch (err) {
        return void 0;
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/sleep.mjs
var sleep;
var init_sleep = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/sleep.mjs"() {
    sleep = (ms, signal) => new Promise((resolve4) => {
      if (signal?.aborted)
        return resolve4();
      const onAbort = () => {
        clearTimeout(timer);
        resolve4();
      };
      const timer = setTimeout(() => {
        signal?.removeEventListener("abort", onAbort);
        resolve4();
      }, ms);
      signal?.addEventListener("abort", onAbort, { once: true });
    });
  }
});

// node_modules/@anthropic-ai/sdk/version.mjs
var VERSION;
var init_version = __esm({
  "node_modules/@anthropic-ai/sdk/version.mjs"() {
    VERSION = "0.116.0";
  }
});

// node_modules/@anthropic-ai/sdk/internal/detect-platform.mjs
function getDetectedPlatform() {
  if (typeof Deno !== "undefined" && Deno.build != null) {
    return "deno";
  }
  if (typeof EdgeRuntime !== "undefined") {
    return "edge";
  }
  if (Object.prototype.toString.call(typeof globalThis.process !== "undefined" ? globalThis.process : 0) === "[object process]") {
    return "node";
  }
  return "unknown";
}
function getBrowserInfo() {
  if (typeof navigator === "undefined" || !navigator) {
    return null;
  }
  const browserPatterns = [
    { key: "edge", pattern: /Edge(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "ie", pattern: /MSIE(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "ie", pattern: /Trident(?:.*rv\:(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "chrome", pattern: /Chrome(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "firefox", pattern: /Firefox(?:\W+(\d+)\.(\d+)(?:\.(\d+))?)?/ },
    { key: "safari", pattern: /(?:Version\W+(\d+)\.(\d+)(?:\.(\d+))?)?(?:\W+Mobile\S*)?\W+Safari/ }
  ];
  for (const { key, pattern } of browserPatterns) {
    const match = pattern.exec(navigator.userAgent);
    if (match) {
      const major = match[1] || 0;
      const minor = match[2] || 0;
      const patch = match[3] || 0;
      return { browser: key, version: `${major}.${minor}.${patch}` };
    }
  }
  return null;
}
var isRunningInBrowser, getPlatformProperties, normalizeArch, normalizePlatform, _platformHeaders, getPlatformHeaders;
var init_detect_platform = __esm({
  "node_modules/@anthropic-ai/sdk/internal/detect-platform.mjs"() {
    init_version();
    isRunningInBrowser = () => {
      return (
        // @ts-ignore
        typeof window !== "undefined" && // @ts-ignore
        typeof window.document !== "undefined" && // @ts-ignore
        typeof navigator !== "undefined"
      );
    };
    getPlatformProperties = () => {
      const detectedPlatform = getDetectedPlatform();
      if (detectedPlatform === "deno") {
        return {
          "X-Stainless-Lang": "js",
          "X-Stainless-Package-Version": VERSION,
          "X-Stainless-OS": normalizePlatform(Deno.build.os),
          "X-Stainless-Arch": normalizeArch(Deno.build.arch),
          "X-Stainless-Runtime": "deno",
          "X-Stainless-Runtime-Version": typeof Deno.version === "string" ? Deno.version : Deno.version?.deno ?? "unknown"
        };
      }
      if (typeof EdgeRuntime !== "undefined") {
        return {
          "X-Stainless-Lang": "js",
          "X-Stainless-Package-Version": VERSION,
          "X-Stainless-OS": "Unknown",
          "X-Stainless-Arch": `other:${EdgeRuntime}`,
          "X-Stainless-Runtime": "edge",
          "X-Stainless-Runtime-Version": globalThis.process.version
        };
      }
      if (detectedPlatform === "node") {
        return {
          "X-Stainless-Lang": "js",
          "X-Stainless-Package-Version": VERSION,
          "X-Stainless-OS": normalizePlatform(globalThis.process.platform ?? "unknown"),
          "X-Stainless-Arch": normalizeArch(globalThis.process.arch ?? "unknown"),
          "X-Stainless-Runtime": "node",
          "X-Stainless-Runtime-Version": globalThis.process.version ?? "unknown"
        };
      }
      const browserInfo = getBrowserInfo();
      if (browserInfo) {
        return {
          "X-Stainless-Lang": "js",
          "X-Stainless-Package-Version": VERSION,
          "X-Stainless-OS": "Unknown",
          "X-Stainless-Arch": "unknown",
          "X-Stainless-Runtime": `browser:${browserInfo.browser}`,
          "X-Stainless-Runtime-Version": browserInfo.version
        };
      }
      return {
        "X-Stainless-Lang": "js",
        "X-Stainless-Package-Version": VERSION,
        "X-Stainless-OS": "Unknown",
        "X-Stainless-Arch": "unknown",
        "X-Stainless-Runtime": "unknown",
        "X-Stainless-Runtime-Version": "unknown"
      };
    };
    normalizeArch = (arch) => {
      if (arch === "x32")
        return "x32";
      if (arch === "x86_64" || arch === "x64")
        return "x64";
      if (arch === "arm")
        return "arm";
      if (arch === "aarch64" || arch === "arm64")
        return "arm64";
      if (arch)
        return `other:${arch}`;
      return "unknown";
    };
    normalizePlatform = (platform) => {
      platform = platform.toLowerCase();
      if (platform.includes("ios"))
        return "iOS";
      if (platform === "android")
        return "Android";
      if (platform === "darwin")
        return "MacOS";
      if (platform === "win32")
        return "Windows";
      if (platform === "freebsd")
        return "FreeBSD";
      if (platform === "openbsd")
        return "OpenBSD";
      if (platform === "linux")
        return "Linux";
      if (platform)
        return `Other:${platform}`;
      return "Unknown";
    };
    getPlatformHeaders = () => {
      return _platformHeaders ?? (_platformHeaders = getPlatformProperties());
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/request-signal.mjs
function makeCleanup(signal, listener) {
  return () => signal.removeEventListener("abort", listener);
}
function registerRequestSignalCleanup(controller, signal, listener) {
  cleanups.set(controller, makeCleanup(signal, listener));
}
function armAbandonmentBackstop(body, controller) {
  if (cleanups.has(controller))
    registry?.register(body, controller, controller);
}
function releaseRequestSignal(controller) {
  const cleanup = cleanups.get(controller);
  if (cleanup) {
    cleanups.delete(controller);
    registry?.unregister(controller);
    cleanup();
  }
}
var cleanups, registry;
var init_request_signal = __esm({
  "node_modules/@anthropic-ai/sdk/internal/request-signal.mjs"() {
    cleanups = /* @__PURE__ */ new WeakMap();
    registry = typeof globalThis.FinalizationRegistry === "function" ? new globalThis.FinalizationRegistry((controller) => releaseRequestSignal(controller)) : null;
  }
});

// node_modules/@anthropic-ai/sdk/internal/shims.mjs
function getDefaultFetch() {
  if (typeof fetch !== "undefined") {
    return fetch;
  }
  throw new Error("`fetch` is not defined as a global; Either pass `fetch` to the client, `new Anthropic({ fetch })` or polyfill the global, `globalThis.fetch = fetch`");
}
function makeReadableStream(...args) {
  const ReadableStream2 = globalThis.ReadableStream;
  if (typeof ReadableStream2 === "undefined") {
    throw new Error("`ReadableStream` is not defined as a global; You will need to polyfill it, `globalThis.ReadableStream = ReadableStream`");
  }
  return new ReadableStream2(...args);
}
function ReadableStreamFrom(iterable) {
  let iter = Symbol.asyncIterator in iterable ? iterable[Symbol.asyncIterator]() : iterable[Symbol.iterator]();
  return makeReadableStream({
    start() {
    },
    async pull(controller) {
      const { done, value } = await iter.next();
      if (done) {
        controller.close();
      } else {
        controller.enqueue(value);
      }
    },
    async cancel() {
      await iter.return?.();
    }
  });
}
function ReadableStreamToAsyncIterable(stream) {
  if (stream[Symbol.asyncIterator])
    return stream;
  const reader = stream.getReader();
  return {
    async next() {
      try {
        const result = await reader.read();
        if (result?.done)
          reader.releaseLock();
        return result;
      } catch (e) {
        reader.releaseLock();
        throw e;
      }
    },
    async return() {
      const cancelPromise = reader.cancel();
      reader.releaseLock();
      await cancelPromise;
      return { done: true, value: void 0 };
    },
    [Symbol.asyncIterator]() {
      return this;
    }
  };
}
async function CancelReadableStream(stream) {
  if (stream === null || typeof stream !== "object")
    return;
  if (stream[Symbol.asyncIterator]) {
    await stream[Symbol.asyncIterator]().return?.();
    return;
  }
  const reader = stream.getReader();
  const cancelPromise = reader.cancel();
  reader.releaseLock();
  await cancelPromise;
}
var init_shims = __esm({
  "node_modules/@anthropic-ai/sdk/internal/shims.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/internal/request-options.mjs
var FallbackEncoder;
var init_request_options = __esm({
  "node_modules/@anthropic-ai/sdk/internal/request-options.mjs"() {
    FallbackEncoder = ({ headers, body }) => {
      return {
        bodyHeaders: {
          "content-type": "application/json"
        },
        body: JSON.stringify(body)
      };
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/qs/formats.mjs
var default_format, default_formatter, formatters, RFC1738;
var init_formats = __esm({
  "node_modules/@anthropic-ai/sdk/internal/qs/formats.mjs"() {
    default_format = "RFC3986";
    default_formatter = (v) => String(v);
    formatters = {
      RFC1738: (v) => String(v).replace(/%20/g, "+"),
      RFC3986: default_formatter
    };
    RFC1738 = "RFC1738";
  }
});

// node_modules/@anthropic-ai/sdk/internal/qs/utils.mjs
function is_buffer(obj) {
  if (!obj || typeof obj !== "object") {
    return false;
  }
  return !!(obj.constructor && obj.constructor.isBuffer && obj.constructor.isBuffer(obj));
}
function maybe_map(val, fn) {
  if (isArray(val)) {
    const mapped = [];
    for (let i = 0; i < val.length; i += 1) {
      mapped.push(fn(val[i]));
    }
    return mapped;
  }
  return fn(val);
}
var has, hex_table, limit, encode;
var init_utils = __esm({
  "node_modules/@anthropic-ai/sdk/internal/qs/utils.mjs"() {
    init_formats();
    init_values();
    has = (obj, key) => (has = Object.hasOwn ?? Function.prototype.call.bind(Object.prototype.hasOwnProperty), has(obj, key));
    hex_table = /* @__PURE__ */ (() => {
      const array = [];
      for (let i = 0; i < 256; ++i) {
        array.push("%" + ((i < 16 ? "0" : "") + i.toString(16)).toUpperCase());
      }
      return array;
    })();
    limit = 1024;
    encode = (str, _defaultEncoder, charset, _kind, format) => {
      if (str.length === 0) {
        return str;
      }
      let string = str;
      if (typeof str === "symbol") {
        string = Symbol.prototype.toString.call(str);
      } else if (typeof str !== "string") {
        string = String(str);
      }
      if (charset === "iso-8859-1") {
        return escape(string).replace(/%u[0-9a-f]{4}/gi, function($0) {
          return "%26%23" + parseInt($0.slice(2), 16) + "%3B";
        });
      }
      let out = "";
      for (let j = 0; j < string.length; j += limit) {
        const segment = string.length >= limit ? string.slice(j, j + limit) : string;
        const arr = [];
        for (let i = 0; i < segment.length; ++i) {
          let c = segment.charCodeAt(i);
          if (c === 45 || // -
          c === 46 || // .
          c === 95 || // _
          c === 126 || // ~
          c >= 48 && c <= 57 || // 0-9
          c >= 65 && c <= 90 || // a-z
          c >= 97 && c <= 122 || // A-Z
          format === RFC1738 && (c === 40 || c === 41)) {
            arr[arr.length] = segment.charAt(i);
            continue;
          }
          if (c < 128) {
            arr[arr.length] = hex_table[c];
            continue;
          }
          if (c < 2048) {
            arr[arr.length] = hex_table[192 | c >> 6] + hex_table[128 | c & 63];
            continue;
          }
          if (c < 55296 || c >= 57344) {
            arr[arr.length] = hex_table[224 | c >> 12] + hex_table[128 | c >> 6 & 63] + hex_table[128 | c & 63];
            continue;
          }
          i += 1;
          c = 65536 + ((c & 1023) << 10 | segment.charCodeAt(i) & 1023);
          arr[arr.length] = hex_table[240 | c >> 18] + hex_table[128 | c >> 12 & 63] + hex_table[128 | c >> 6 & 63] + hex_table[128 | c & 63];
        }
        out += arr.join("");
      }
      return out;
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/qs/stringify.mjs
function is_non_nullish_primitive(v) {
  return typeof v === "string" || typeof v === "number" || typeof v === "boolean" || typeof v === "symbol" || typeof v === "bigint";
}
function inner_stringify(object, prefix, generateArrayPrefix, commaRoundTrip, allowEmptyArrays, strictNullHandling, skipNulls, encodeDotInKeys, encoder2, filter, sort, allowDots, serializeDate, format, formatter, encodeValuesOnly, charset, sideChannel) {
  let obj = object;
  let tmp_sc = sideChannel;
  let step = 0;
  let find_flag = false;
  while ((tmp_sc = tmp_sc.get(sentinel)) !== void 0 && !find_flag) {
    const pos = tmp_sc.get(object);
    step += 1;
    if (typeof pos !== "undefined") {
      if (pos === step) {
        throw new RangeError("Cyclic object value");
      } else {
        find_flag = true;
      }
    }
    if (typeof tmp_sc.get(sentinel) === "undefined") {
      step = 0;
    }
  }
  if (typeof filter === "function") {
    obj = filter(prefix, obj);
  } else if (obj instanceof Date) {
    obj = serializeDate?.(obj);
  } else if (generateArrayPrefix === "comma" && isArray(obj)) {
    obj = maybe_map(obj, function(value) {
      if (value instanceof Date) {
        return serializeDate?.(value);
      }
      return value;
    });
  }
  if (obj === null) {
    if (strictNullHandling) {
      return encoder2 && !encodeValuesOnly ? (
        // @ts-expect-error
        encoder2(prefix, defaults.encoder, charset, "key", format)
      ) : prefix;
    }
    obj = "";
  }
  if (is_non_nullish_primitive(obj) || is_buffer(obj)) {
    if (encoder2) {
      const key_value = encodeValuesOnly ? prefix : encoder2(prefix, defaults.encoder, charset, "key", format);
      return [
        formatter?.(key_value) + "=" + // @ts-expect-error
        formatter?.(encoder2(obj, defaults.encoder, charset, "value", format))
      ];
    }
    return [formatter?.(prefix) + "=" + formatter?.(String(obj))];
  }
  const values = [];
  if (typeof obj === "undefined") {
    return values;
  }
  let obj_keys;
  if (generateArrayPrefix === "comma" && isArray(obj)) {
    if (encodeValuesOnly && encoder2) {
      obj = maybe_map(obj, encoder2);
    }
    obj_keys = [{ value: obj.length > 0 ? obj.join(",") || null : void 0 }];
  } else if (isArray(filter)) {
    obj_keys = filter;
  } else {
    const keys = Object.keys(obj);
    obj_keys = sort ? keys.sort(sort) : keys;
  }
  const encoded_prefix = encodeDotInKeys ? String(prefix).replace(/\./g, "%2E") : String(prefix);
  const adjusted_prefix = commaRoundTrip && isArray(obj) && obj.length === 1 ? encoded_prefix + "[]" : encoded_prefix;
  if (allowEmptyArrays && isArray(obj) && obj.length === 0) {
    return adjusted_prefix + "[]";
  }
  for (let j = 0; j < obj_keys.length; ++j) {
    const key = obj_keys[j];
    const value = (
      // @ts-ignore
      typeof key === "object" && typeof key.value !== "undefined" ? key.value : obj[key]
    );
    if (skipNulls && value === null) {
      continue;
    }
    const encoded_key = allowDots && encodeDotInKeys ? key.replace(/\./g, "%2E") : key;
    const key_prefix = isArray(obj) ? typeof generateArrayPrefix === "function" ? generateArrayPrefix(adjusted_prefix, encoded_key) : adjusted_prefix : adjusted_prefix + (allowDots ? "." + encoded_key : "[" + encoded_key + "]");
    sideChannel.set(object, step);
    const valueSideChannel = /* @__PURE__ */ new WeakMap();
    valueSideChannel.set(sentinel, sideChannel);
    push_to_array(values, inner_stringify(
      value,
      key_prefix,
      generateArrayPrefix,
      commaRoundTrip,
      allowEmptyArrays,
      strictNullHandling,
      skipNulls,
      encodeDotInKeys,
      // @ts-ignore
      generateArrayPrefix === "comma" && encodeValuesOnly && isArray(obj) ? null : encoder2,
      filter,
      sort,
      allowDots,
      serializeDate,
      format,
      formatter,
      encodeValuesOnly,
      charset,
      valueSideChannel
    ));
  }
  return values;
}
function normalize_stringify_options(opts = defaults) {
  if (typeof opts.allowEmptyArrays !== "undefined" && typeof opts.allowEmptyArrays !== "boolean") {
    throw new TypeError("`allowEmptyArrays` option can only be `true` or `false`, when provided");
  }
  if (typeof opts.encodeDotInKeys !== "undefined" && typeof opts.encodeDotInKeys !== "boolean") {
    throw new TypeError("`encodeDotInKeys` option can only be `true` or `false`, when provided");
  }
  if (opts.encoder !== null && typeof opts.encoder !== "undefined" && typeof opts.encoder !== "function") {
    throw new TypeError("Encoder has to be a function.");
  }
  const charset = opts.charset || defaults.charset;
  if (typeof opts.charset !== "undefined" && opts.charset !== "utf-8" && opts.charset !== "iso-8859-1") {
    throw new TypeError("The charset option must be either utf-8, iso-8859-1, or undefined");
  }
  let format = default_format;
  if (typeof opts.format !== "undefined") {
    if (!has(formatters, opts.format)) {
      throw new TypeError("Unknown format option provided.");
    }
    format = opts.format;
  }
  const formatter = formatters[format];
  let filter = defaults.filter;
  if (typeof opts.filter === "function" || isArray(opts.filter)) {
    filter = opts.filter;
  }
  let arrayFormat;
  if (opts.arrayFormat && opts.arrayFormat in array_prefix_generators) {
    arrayFormat = opts.arrayFormat;
  } else if ("indices" in opts) {
    arrayFormat = opts.indices ? "indices" : "repeat";
  } else {
    arrayFormat = defaults.arrayFormat;
  }
  if ("commaRoundTrip" in opts && typeof opts.commaRoundTrip !== "boolean") {
    throw new TypeError("`commaRoundTrip` must be a boolean, or absent");
  }
  const allowDots = typeof opts.allowDots === "undefined" ? !!opts.encodeDotInKeys === true ? true : defaults.allowDots : !!opts.allowDots;
  return {
    addQueryPrefix: typeof opts.addQueryPrefix === "boolean" ? opts.addQueryPrefix : defaults.addQueryPrefix,
    // @ts-ignore
    allowDots,
    allowEmptyArrays: typeof opts.allowEmptyArrays === "boolean" ? !!opts.allowEmptyArrays : defaults.allowEmptyArrays,
    arrayFormat,
    charset,
    charsetSentinel: typeof opts.charsetSentinel === "boolean" ? opts.charsetSentinel : defaults.charsetSentinel,
    commaRoundTrip: !!opts.commaRoundTrip,
    delimiter: typeof opts.delimiter === "undefined" ? defaults.delimiter : opts.delimiter,
    encode: typeof opts.encode === "boolean" ? opts.encode : defaults.encode,
    encodeDotInKeys: typeof opts.encodeDotInKeys === "boolean" ? opts.encodeDotInKeys : defaults.encodeDotInKeys,
    encoder: typeof opts.encoder === "function" ? opts.encoder : defaults.encoder,
    encodeValuesOnly: typeof opts.encodeValuesOnly === "boolean" ? opts.encodeValuesOnly : defaults.encodeValuesOnly,
    filter,
    format,
    formatter,
    serializeDate: typeof opts.serializeDate === "function" ? opts.serializeDate : defaults.serializeDate,
    skipNulls: typeof opts.skipNulls === "boolean" ? opts.skipNulls : defaults.skipNulls,
    // @ts-ignore
    sort: typeof opts.sort === "function" ? opts.sort : null,
    strictNullHandling: typeof opts.strictNullHandling === "boolean" ? opts.strictNullHandling : defaults.strictNullHandling
  };
}
function stringify(object, opts = {}) {
  let obj = object;
  const options = normalize_stringify_options(opts);
  let obj_keys;
  let filter;
  if (typeof options.filter === "function") {
    filter = options.filter;
    obj = filter("", obj);
  } else if (isArray(options.filter)) {
    filter = options.filter;
    obj_keys = filter;
  }
  const keys = [];
  if (typeof obj !== "object" || obj === null) {
    return "";
  }
  const generateArrayPrefix = array_prefix_generators[options.arrayFormat];
  const commaRoundTrip = generateArrayPrefix === "comma" && options.commaRoundTrip;
  if (!obj_keys) {
    obj_keys = Object.keys(obj);
  }
  if (options.sort) {
    obj_keys.sort(options.sort);
  }
  const sideChannel = /* @__PURE__ */ new WeakMap();
  for (let i = 0; i < obj_keys.length; ++i) {
    const key = obj_keys[i];
    if (options.skipNulls && obj[key] === null) {
      continue;
    }
    push_to_array(keys, inner_stringify(
      obj[key],
      key,
      // @ts-expect-error
      generateArrayPrefix,
      commaRoundTrip,
      options.allowEmptyArrays,
      options.strictNullHandling,
      options.skipNulls,
      options.encodeDotInKeys,
      options.encode ? options.encoder : null,
      options.filter,
      options.sort,
      options.allowDots,
      options.serializeDate,
      options.format,
      options.formatter,
      options.encodeValuesOnly,
      options.charset,
      sideChannel
    ));
  }
  const joined = keys.join(options.delimiter);
  let prefix = options.addQueryPrefix === true ? "?" : "";
  if (options.charsetSentinel) {
    if (options.charset === "iso-8859-1") {
      prefix += "utf8=%26%2310003%3B&";
    } else {
      prefix += "utf8=%E2%9C%93&";
    }
  }
  return joined.length > 0 ? prefix + joined : "";
}
var array_prefix_generators, push_to_array, toISOString, defaults, sentinel;
var init_stringify = __esm({
  "node_modules/@anthropic-ai/sdk/internal/qs/stringify.mjs"() {
    init_utils();
    init_formats();
    init_values();
    array_prefix_generators = {
      brackets(prefix) {
        return String(prefix) + "[]";
      },
      comma: "comma",
      indices(prefix, key) {
        return String(prefix) + "[" + key + "]";
      },
      repeat(prefix) {
        return String(prefix);
      }
    };
    push_to_array = function(arr, value_or_array) {
      Array.prototype.push.apply(arr, isArray(value_or_array) ? value_or_array : [value_or_array]);
    };
    defaults = {
      addQueryPrefix: false,
      allowDots: false,
      allowEmptyArrays: false,
      arrayFormat: "indices",
      charset: "utf-8",
      charsetSentinel: false,
      delimiter: "&",
      encode: true,
      encodeDotInKeys: false,
      encoder: encode,
      encodeValuesOnly: false,
      format: default_format,
      formatter: default_formatter,
      /** @deprecated */
      indices: false,
      serializeDate(date) {
        return (toISOString ?? (toISOString = Function.prototype.call.bind(Date.prototype.toISOString)))(date);
      },
      skipNulls: false,
      strictNullHandling: false
    };
    sentinel = {};
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/query.mjs
function stringifyQuery(query) {
  return stringify(query, { arrayFormat: "brackets" });
}
var init_query = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/query.mjs"() {
    init_stringify();
  }
});

// node_modules/@anthropic-ai/sdk/lib/credentials/types.mjs
function requireSecureTokenEndpoint(baseURL) {
  if (!baseURL)
    return;
  let u;
  try {
    u = new URL(baseURL);
  } catch (err) {
    throw new WorkloadIdentityError(`Invalid token endpoint base URL "${baseURL}": ${err}`);
  }
  if (u.protocol === "https:")
    return;
  const host = u.hostname.toLowerCase().replace(/^\[|\]$/g, "");
  if (u.protocol === "http:" && (host === "localhost" || host === "127.0.0.1" || host === "::1")) {
    return;
  }
  throw new WorkloadIdentityError(`Refusing to send credential over non-https token endpoint "${baseURL}"`);
}
async function parseTokenResponse(resp, requestId) {
  const text = await readLimitedText(resp);
  let data;
  try {
    data = JSON.parse(text);
  } catch {
    throw new WorkloadIdentityError(`Token endpoint returned non-JSON response (status ${resp.status})`, resp.status, redactSensitive(text), requestId);
  }
  if (!data.access_token) {
    throw new WorkloadIdentityError(`Token endpoint response missing access_token: ${JSON.stringify(redactSensitive(data))}`, resp.status, redactSensitive(data), requestId);
  }
  if (data.token_type && data.token_type.toLowerCase() !== "bearer") {
    throw new WorkloadIdentityError(`Token endpoint response: unsupported token_type "${data.token_type}" (want Bearer)`, resp.status, redactSensitive(data), requestId);
  }
  return data;
}
function redactSensitive(body) {
  if (body == null)
    return body;
  if (typeof body === "string") {
    let parsed;
    try {
      parsed = JSON.parse(body);
    } catch {
      if (body.length <= MAX_ERROR_BODY_CHARS)
        return body;
      return body.slice(0, MAX_ERROR_BODY_CHARS) + `... <${body.length - MAX_ERROR_BODY_CHARS} more chars>`;
    }
    return JSON.stringify(redactSensitive(parsed));
  }
  if (typeof body === "object" && !Array.isArray(body)) {
    const out = {};
    for (const [k, v] of Object.entries(body)) {
      if (SAFE_ERROR_KEYS.has(k))
        out[k] = v;
    }
    return out;
  }
  return null;
}
async function checkCredentialsFileSafety(path6, onWarn = (m) => console.warn(`anthropic-sdk: ${m}`)) {
  if (typeof process === "undefined" || process.platform === "win32")
    return;
  const fs5 = await import("node:fs");
  let resolved = path6;
  let st;
  try {
    resolved = await fs5.promises.realpath(path6);
    st = await fs5.promises.stat(resolved);
  } catch {
    return;
  }
  const mode = st.mode & 511;
  if (mode & 18) {
    throw new WorkloadIdentityError(`Credentials file at ${resolved} is group/world-writable (mode 0o${mode.toString(8)}); this allows other local users to plant tokens. Run \`chmod 600 ${resolved}\`.`);
  }
  if (mode & 36) {
    throw new WorkloadIdentityError(`Credentials file at ${resolved} is group/world-readable (mode 0o${mode.toString(8)}); run \`chmod 600 ${resolved}\` before retrying.`);
  }
  if (typeof process.getuid === "function" && st.uid !== process.getuid()) {
    onWarn(`credentials file at ${resolved} is owned by uid ${st.uid} (current process uid ${process.getuid()}); verify this is intentional.`);
  }
}
async function writeCredentialsFileAtomic(targetPath, data) {
  const fs5 = await import("node:fs");
  const path6 = await import("node:path");
  const dir = path6.dirname(targetPath);
  await fs5.promises.mkdir(dir, { recursive: true, mode: 448 });
  const tmpPath = `${targetPath}.${process.pid}.${Math.random().toString(36).slice(2)}.tmp`;
  try {
    const fh = await fs5.promises.open(tmpPath, "w", 384);
    try {
      await fh.writeFile(JSON.stringify(data, null, 2));
      await fh.sync();
    } finally {
      await fh.close();
    }
    await fs5.promises.rename(tmpPath, targetPath);
  } catch (err) {
    await fs5.promises.unlink(tmpPath).catch(() => {
    });
    throw err;
  }
  try {
    const dirFh = await fs5.promises.open(dir, "r");
    try {
      await dirFh.sync();
    } finally {
      await dirFh.close();
    }
  } catch {
  }
}
async function readLimitedText(resp) {
  if (!resp.body) {
    return "";
  }
  const reader = resp.body.getReader();
  const chunks = [];
  let received = 0;
  for (; ; ) {
    const { done, value } = await reader.read();
    if (done)
      break;
    if (received + value.length > MAX_TOKEN_RESPONSE_BYTES) {
      const remaining = MAX_TOKEN_RESPONSE_BYTES - received;
      if (remaining > 0)
        chunks.push(value.subarray(0, remaining));
      await reader.cancel();
      break;
    }
    chunks.push(value);
    received += value.length;
  }
  let merged;
  if (chunks.length === 1) {
    merged = chunks[0];
  } else {
    merged = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0));
    let offset = 0;
    for (const c of chunks) {
      merged.set(c, offset);
      offset += c.length;
    }
  }
  return new TextDecoder("utf-8").decode(merged);
}
var GRANT_TYPE_JWT_BEARER, GRANT_TYPE_REFRESH_TOKEN, TOKEN_ENDPOINT, OAUTH_API_BETA_HEADER, FEDERATION_BETA_HEADER, ADVISORY_REFRESH_THRESHOLD_IN_SECONDS, MANDATORY_REFRESH_THRESHOLD_IN_SECONDS, ADVISORY_REFRESH_BACKOFF_IN_SECONDS, MAX_TOKEN_RESPONSE_BYTES, MAX_ERROR_BODY_CHARS, SAFE_ERROR_KEYS, WorkloadIdentityError;
var init_types = __esm({
  "node_modules/@anthropic-ai/sdk/lib/credentials/types.mjs"() {
    init_error();
    GRANT_TYPE_JWT_BEARER = "urn:ietf:params:oauth:grant-type:jwt-bearer";
    GRANT_TYPE_REFRESH_TOKEN = "refresh_token";
    TOKEN_ENDPOINT = "/v1/oauth/token";
    OAUTH_API_BETA_HEADER = "oauth-2025-04-20";
    FEDERATION_BETA_HEADER = "oidc-federation-2026-04-01";
    ADVISORY_REFRESH_THRESHOLD_IN_SECONDS = 120;
    MANDATORY_REFRESH_THRESHOLD_IN_SECONDS = 30;
    ADVISORY_REFRESH_BACKOFF_IN_SECONDS = 5;
    MAX_TOKEN_RESPONSE_BYTES = 1 << 20;
    MAX_ERROR_BODY_CHARS = 2e3;
    SAFE_ERROR_KEYS = /* @__PURE__ */ new Set(["error", "error_description", "error_uri"]);
    WorkloadIdentityError = class extends AnthropicError {
      constructor(message, statusCode = null, body = null, requestId = null) {
        super(message);
        this.statusCode = statusCode;
        this.body = body;
        this.requestId = requestId;
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/time.mjs
function nowAsSeconds() {
  return Math.floor(Date.now() / 1e3);
}
var init_time = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/time.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/lib/credentials/token-cache.mjs
var TokenCache;
var init_token_cache = __esm({
  "node_modules/@anthropic-ai/sdk/lib/credentials/token-cache.mjs"() {
    init_types();
    init_time();
    TokenCache = class {
      constructor(provider2, onAdvisoryRefreshError) {
        this.cached = null;
        this.pendingRefresh = null;
        this.nextForce = false;
        this.lastAdvisoryError = 0;
        this.provider = provider2;
        this.onAdvisoryRefreshError = onAdvisoryRefreshError;
      }
      async getToken() {
        const force = this.nextForce;
        this.nextForce = false;
        const cached = this.cached;
        if (force || cached == null) {
          const token3 = await this.refresh(force);
          return token3.token;
        }
        if (cached.expiresAt == null) {
          return cached.token;
        }
        const remaining = cached.expiresAt - nowAsSeconds();
        if (remaining > ADVISORY_REFRESH_THRESHOLD_IN_SECONDS) {
          return cached.token;
        }
        if (remaining > MANDATORY_REFRESH_THRESHOLD_IN_SECONDS) {
          this.backgroundRefresh();
          return cached.token;
        }
        const token2 = await this.refresh();
        return token2.token;
      }
      /**
       * Clears the cached token and marks the next {@link getToken} as a forced
       * refresh, so the underlying provider bypasses any on-disk freshness check.
       * Called after a 401 — the server has just told us the token is bad even
       * if its `expires_at` still looks fresh.
       */
      invalidate() {
        this.cached = null;
        this.nextForce = true;
      }
      /**
       * Mandatory refresh. Joins any in-flight refresh unless forced — a forced
       * refresh must not coalesce into a non-forced one that may re-serve the
       * same stale disk token.
       */
      refresh(force = false) {
        if (this.pendingRefresh && !force) {
          return this.pendingRefresh;
        }
        return this.doRefresh(force);
      }
      /**
       * Advisory background refresh. Shares the same in-flight promise as
       * mandatory refreshes for deduplication, but swallows errors so the
       * stale cached token keeps being served. Backs off for
       * {@link ADVISORY_REFRESH_BACKOFF_IN_SECONDS} after a failure so an
       * outage during the advisory window doesn't hammer the token endpoint.
       */
      backgroundRefresh() {
        if (this.pendingRefresh) {
          return;
        }
        if (nowAsSeconds() - this.lastAdvisoryError < ADVISORY_REFRESH_BACKOFF_IN_SECONDS) {
          return;
        }
        this.doRefresh().catch((err) => {
          this.lastAdvisoryError = nowAsSeconds();
          this.onAdvisoryRefreshError?.(err);
        });
      }
      /**
       * Core refresh. Sets {@link pendingRefresh} so concurrent callers
       * (both advisory and mandatory) coalesce into a single provider call.
       */
      doRefresh(force = false) {
        this.pendingRefresh = this.provider(force ? { forceRefresh: true } : void 0).then((token2) => {
          this.cached = token2;
          this.pendingRefresh = null;
          return token2;
        }, (err) => {
          this.pendingRefresh = null;
          throw err;
        });
        return this.pendingRefresh;
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/env.mjs
var readEnv;
var init_env = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/env.mjs"() {
    readEnv = (env) => {
      if (typeof globalThis.process !== "undefined") {
        return globalThis.process.env?.[env]?.trim() || void 0;
      }
      if (typeof globalThis.Deno !== "undefined") {
        return globalThis.Deno.env?.get?.(env)?.trim() || void 0;
      }
      return void 0;
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/bytes.mjs
function concatBytes(buffers) {
  let length = 0;
  for (const buffer of buffers) {
    length += buffer.length;
  }
  const output = new Uint8Array(length);
  let index = 0;
  for (const buffer of buffers) {
    output.set(buffer, index);
    index += buffer.length;
  }
  return output;
}
function encodeUTF8(str) {
  let encoder2;
  return (encodeUTF8_ ?? (encoder2 = new globalThis.TextEncoder(), encodeUTF8_ = encoder2.encode.bind(encoder2)))(str);
}
function decodeUTF8(bytes) {
  let decoder;
  return (decodeUTF8_ ?? (decoder = new globalThis.TextDecoder(), decodeUTF8_ = decoder.decode.bind(decoder)))(bytes);
}
var encodeUTF8_, decodeUTF8_;
var init_bytes = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/bytes.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/base64.mjs
var init_base64 = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/base64.mjs"() {
    init_error();
    init_bytes();
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/log.mjs
function noop() {
}
function makeLogFn(fnLevel, logger, logLevel) {
  if (!logger || levelNumbers[fnLevel] > levelNumbers[logLevel]) {
    return noop;
  } else {
    return logger[fnLevel].bind(logger);
  }
}
function filterLogger(logger, logLevel) {
  const cachedLogger = cachedLoggers.get(logger);
  if (cachedLogger && cachedLogger[0] === logLevel) {
    return cachedLogger[1];
  }
  const levelLogger = {
    error: makeLogFn("error", logger, logLevel),
    warn: makeLogFn("warn", logger, logLevel),
    info: makeLogFn("info", logger, logLevel),
    debug: makeLogFn("debug", logger, logLevel)
  };
  cachedLoggers.set(logger, [logLevel, levelLogger]);
  return levelLogger;
}
function loggerFor(client) {
  const logger = client.logger;
  const logLevel = client.logLevel ?? "off";
  if (!logger) {
    return noopLogger;
  }
  return filterLogger(logger, logLevel);
}
function defaultLogger() {
  const envLevel = readEnv("ANTHROPIC_LOG");
  if (!cachedDefaultLogger || envLevel !== lastEnvLevel) {
    lastEnvLevel = envLevel;
    cachedDefaultLogger = filterLogger(console, parseLogLevel(envLevel, "process.env['ANTHROPIC_LOG']", filterLogger(console, defaultLogLevel)) ?? defaultLogLevel);
  }
  return cachedDefaultLogger;
}
var defaultLogLevel, levelNumbers, parseLogLevel, noopLogger, cachedLoggers, lastEnvLevel, cachedDefaultLogger, formatRequestDetails;
var init_log = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/log.mjs"() {
    init_values();
    init_env();
    defaultLogLevel = "warn";
    levelNumbers = {
      off: 0,
      error: 200,
      warn: 300,
      info: 400,
      debug: 500
    };
    parseLogLevel = (maybeLevel, sourceName, logger) => {
      if (!maybeLevel) {
        return void 0;
      }
      if (hasOwn(levelNumbers, maybeLevel)) {
        return maybeLevel;
      }
      logger.warn(`${sourceName} was set to ${JSON.stringify(maybeLevel)}, expected one of ${JSON.stringify(Object.keys(levelNumbers))}`);
      return void 0;
    };
    noopLogger = {
      error: noop,
      warn: noop,
      info: noop,
      debug: noop
    };
    cachedLoggers = /* @__PURE__ */ new WeakMap();
    formatRequestDetails = (details) => {
      if (details.options) {
        details.options = { ...details.options };
        delete details.options["headers"];
      }
      if (details.headers) {
        details.headers = Object.fromEntries((details.headers instanceof Headers ? [...details.headers] : Object.entries(details.headers)).map(([name, value]) => [
          name,
          name.toLowerCase() === "authorization" || name.toLowerCase() === "api-key" || name.toLowerCase() === "x-api-key" || name.toLowerCase() === "cookie" || name.toLowerCase() === "set-cookie" ? "***" : value
        ]));
      }
      if ("retryOfRequestLogID" in details) {
        if (details.retryOfRequestLogID) {
          details.retryOf = details.retryOfRequestLogID;
        }
        delete details.retryOfRequestLogID;
      }
      return details;
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils.mjs
var init_utils2 = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils.mjs"() {
    init_values();
    init_base64();
    init_env();
    init_log();
    init_uuid();
    init_sleep();
    init_query();
  }
});

// node_modules/@anthropic-ai/sdk/core/credentials.mjs
function validateProfileName(name) {
  if (!name) {
    throw new Error("profile name is empty");
  }
  if (name === "." || name === "..") {
    throw new Error(`profile name "${name}" is not allowed`);
  }
  if (name.includes("/") || name.includes("\\")) {
    throw new Error(`profile name "${name}" must not contain path separators`);
  }
  if (!PROFILE_NAME_PATTERN.test(name)) {
    throw new Error(`profile name "${name}" contains disallowed characters (allowed: letters, digits, '_', '.', '-')`);
  }
}
var CREDENTIALS_FILE_VERSION, PROFILE_NAME_PATTERN, loadConfigWithSource, getCredentialsPath, getRootConfigPath, supportsLocalConfigFiles, getActiveProfileName;
var init_credentials = __esm({
  "node_modules/@anthropic-ai/sdk/core/credentials.mjs"() {
    init_detect_platform();
    init_utils2();
    CREDENTIALS_FILE_VERSION = "1.0";
    PROFILE_NAME_PATTERN = /^[A-Za-z0-9_.-]+$/;
    loadConfigWithSource = async (profile) => {
      var _a2, _b;
      const rootConfigPath = await getRootConfigPath();
      if (rootConfigPath === null) {
        return null;
      }
      const profileName = profile ?? await getActiveProfileName();
      if (profileName === null) {
        return null;
      }
      validateProfileName(profileName);
      const fs5 = await import("node:fs");
      const path6 = await import("node:path");
      const configPath = path6.join(rootConfigPath, "configs", `${profileName}.json`);
      let configRaw;
      try {
        configRaw = await fs5.promises.readFile(configPath, "utf-8");
      } catch (err) {
        if (err?.code !== "ENOENT") {
          throw new Error(`failed to read config file ${configPath}: ${err}`);
        }
        configRaw = null;
      }
      if (configRaw === null) {
        const organizationId = readEnv("ANTHROPIC_ORGANIZATION_ID");
        const identityTokenFile = readEnv("ANTHROPIC_IDENTITY_TOKEN_FILE");
        const federationRuleId = readEnv("ANTHROPIC_FEDERATION_RULE_ID");
        if (federationRuleId && organizationId) {
          return {
            fromFile: false,
            config: {
              organization_id: organizationId,
              // A defaulted-but-empty CI variable (`ANTHROPIC_WORKSPACE_ID=""`) is
              // treated as unset — readEnv coerces empty to undefined, and the body
              // builder's truthy check skips it — so `"workspace_id": ""` never goes
              // on the wire.
              workspace_id: readEnv("ANTHROPIC_WORKSPACE_ID"),
              base_url: readEnv("ANTHROPIC_BASE_URL"),
              authentication: {
                type: "oidc_federation",
                federation_rule_id: federationRuleId,
                service_account_id: readEnv("ANTHROPIC_SERVICE_ACCOUNT_ID"),
                identity_token: identityTokenFile ? { source: "file", path: identityTokenFile } : void 0,
                scope: readEnv("ANTHROPIC_SCOPE")
              }
            }
          };
        }
        return null;
      }
      let config;
      try {
        config = JSON.parse(configRaw);
      } catch (err) {
        throw new Error(`failed to parse config file ${configPath}: ${err}`);
      }
      if (!config.authentication) {
        throw new Error(`config file ${configPath} is missing "authentication"`);
      }
      const authType = config.authentication.type;
      if (authType !== "oidc_federation" && authType !== "user_oauth") {
        throw new Error(`authentication.type "${authType}" is not a known authentication type`);
      }
      config.organization_id ?? (config.organization_id = readEnv("ANTHROPIC_ORGANIZATION_ID"));
      config.workspace_id ?? (config.workspace_id = readEnv("ANTHROPIC_WORKSPACE_ID"));
      config.base_url ?? (config.base_url = readEnv("ANTHROPIC_BASE_URL"));
      (_a2 = config.authentication).scope ?? (_a2.scope = readEnv("ANTHROPIC_SCOPE"));
      if (config.authentication.type === "oidc_federation") {
        if (!config.authentication.identity_token) {
          const identityTokenFile = readEnv("ANTHROPIC_IDENTITY_TOKEN_FILE");
          if (identityTokenFile) {
            config.authentication.identity_token = {
              source: "file",
              path: identityTokenFile
            };
          }
        }
        if (!config.authentication.federation_rule_id) {
          config.authentication.federation_rule_id = readEnv("ANTHROPIC_FEDERATION_RULE_ID") ?? "";
        }
        (_b = config.authentication).service_account_id ?? (_b.service_account_id = readEnv("ANTHROPIC_SERVICE_ACCOUNT_ID"));
      }
      return { config, fromFile: true };
    };
    getCredentialsPath = async (config, profile) => {
      if (config?.authentication.credentials_path) {
        return config.authentication.credentials_path;
      }
      const rootConfigPath = await getRootConfigPath();
      if (!rootConfigPath) {
        return null;
      }
      const profileName = profile ?? await getActiveProfileName();
      if (!profileName) {
        return null;
      }
      validateProfileName(profileName);
      const path6 = await import("node:path");
      return path6.join(rootConfigPath, "credentials", `${profileName}.json`);
    };
    getRootConfigPath = async () => {
      if (!supportsLocalConfigFiles()) {
        return null;
      }
      const path6 = await import("node:path");
      const configDir = readEnv("ANTHROPIC_CONFIG_DIR");
      if (configDir) {
        return configDir;
      }
      const os = getPlatformHeaders()["X-Stainless-OS"];
      if (os === "Windows") {
        const appData = readEnv("APPDATA");
        if (appData) {
          return path6.join(appData, "Anthropic");
        }
        const userProfile = readEnv("USERPROFILE");
        if (userProfile) {
          return path6.join(userProfile, "AppData", "Roaming", "Anthropic");
        }
        return null;
      }
      const xdgConfigHome = readEnv("XDG_CONFIG_HOME");
      if (xdgConfigHome) {
        return path6.join(xdgConfigHome, "anthropic");
      }
      const home = readEnv("HOME");
      if (home) {
        return path6.join(home, ".config", "anthropic");
      }
      return null;
    };
    supportsLocalConfigFiles = () => {
      const runtime = getPlatformHeaders()["X-Stainless-Runtime"];
      return runtime === "node" || runtime === "deno";
    };
    getActiveProfileName = async () => {
      const rootConfigPath = await getRootConfigPath();
      if (!rootConfigPath) {
        return null;
      }
      const profileName = readEnv("ANTHROPIC_PROFILE");
      if (profileName) {
        return profileName;
      }
      const fs5 = await import("node:fs");
      const path6 = await import("node:path");
      const filePath = path6.join(rootConfigPath, "active_config");
      try {
        return (await fs5.promises.readFile(filePath, "utf-8")).trim() || "default";
      } catch (err) {
        if (err?.code !== "ENOENT") {
          throw new Error(`failed to read ${filePath}: ${err}`);
        }
        return "default";
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/lib/credentials/identity-token.mjs
function identityTokenFromFile(path6) {
  if (!path6) {
    throw new AnthropicError("Identity token file path is empty");
  }
  return async () => {
    const fs5 = await import("node:fs");
    let content;
    try {
      content = await fs5.promises.readFile(path6, "utf-8");
    } catch (err) {
      throw new AnthropicError(`Failed to read identity token file at ${path6}: ${err}`);
    }
    const token2 = content.trim();
    if (!token2) {
      throw new AnthropicError(`Identity token file at ${path6} is empty`);
    }
    return token2;
  };
}
function identityTokenFromValue(token2) {
  if (!token2) {
    throw new AnthropicError("Identity token value is empty");
  }
  return () => token2;
}
var init_identity_token = __esm({
  "node_modules/@anthropic-ai/sdk/lib/credentials/identity-token.mjs"() {
    init_error();
  }
});

// node_modules/@anthropic-ai/sdk/lib/credentials/oidc-federation.mjs
function oidcFederationProvider(config) {
  return async () => {
    requireSecureTokenEndpoint(config.baseURL);
    const jwt = await config.identityTokenProvider();
    if (jwt.length > 16 * 1024) {
      throw new WorkloadIdentityError(`Identity token is ${Math.ceil(jwt.length / 1024)} KiB, exceeds the 16 KiB assertion limit`);
    }
    const body = {
      grant_type: GRANT_TYPE_JWT_BEARER,
      assertion: jwt,
      federation_rule_id: config.federationRuleId,
      organization_id: config.organizationId
    };
    if (config.serviceAccountId) {
      body["service_account_id"] = config.serviceAccountId;
    }
    if (config.workspaceId) {
      body["workspace_id"] = config.workspaceId;
    }
    const url = `${config.baseURL}${TOKEN_ENDPOINT}`;
    let resp;
    try {
      resp = await config.fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "anthropic-beta": `${OAUTH_API_BETA_HEADER},${FEDERATION_BETA_HEADER}`,
          "User-Agent": config.userAgent || `anthropic-sdk-typescript/${VERSION} oidcFederationProvider`
        },
        body: JSON.stringify(body)
      });
    } catch (err) {
      throw new WorkloadIdentityError(`Failed to reach token endpoint ${url}: ${err}`);
    }
    const requestId = resp.headers.get("Request-Id");
    if (!resp.ok) {
      const text = await resp.text().catch(() => "");
      const redacted = redactSensitive(text);
      let hint = "";
      if (resp.status === 401) {
        const hintMiddle = config.workspaceId ? "" : "If your federation rule is scoped to multiple workspaces, set the ANTHROPIC_WORKSPACE_ID environment variable, the 'workspace_id' config key, or the `workspaceId` option. ";
        hint = ` Ensure your federation rule matches your identity token. ${hintMiddle}View your authentication events in the Workload identity page of Claude Console for more details.`;
      }
      throw new WorkloadIdentityError(`Token exchange failed with status ${resp.status}${requestId ? ` (request-id ${requestId})` : ""}: ${redacted}${hint}`, resp.status, redacted, requestId);
    }
    const data = await parseTokenResponse(resp, requestId);
    const expiresIn = Number(data.expires_in);
    if (!Number.isFinite(expiresIn)) {
      throw new WorkloadIdentityError(`Token endpoint response missing required fields: ${JSON.stringify(redactSensitive(data))}`, resp.status, redactSensitive(data), requestId);
    }
    return {
      token: data.access_token,
      expiresAt: nowAsSeconds() + expiresIn
    };
  };
}
var init_oidc_federation = __esm({
  "node_modules/@anthropic-ai/sdk/lib/credentials/oidc-federation.mjs"() {
    init_types();
    init_time();
    init_version();
  }
});

// node_modules/@anthropic-ai/sdk/lib/credentials/user-oauth.mjs
function userOAuthProvider(config) {
  return async (opts) => {
    const fs5 = await import("node:fs");
    await checkCredentialsFileSafety(config.credentialsPath, config.onSafetyWarning);
    let raw;
    try {
      raw = await fs5.promises.readFile(config.credentialsPath, "utf-8");
    } catch (err) {
      throw new WorkloadIdentityError(`Credentials file not found at ${config.credentialsPath}: ${err}`);
    }
    let creds;
    try {
      creds = JSON.parse(raw);
    } catch (err) {
      throw new WorkloadIdentityError(`Credentials file at ${config.credentialsPath} is not valid JSON: ${err}`);
    }
    const accessToken = creds.access_token;
    if (!accessToken) {
      throw new WorkloadIdentityError(`Credentials file at ${config.credentialsPath} must include 'access_token'`);
    }
    const expiresAt = creds.expires_at;
    if (!opts?.forceRefresh && (expiresAt == null || nowAsSeconds() < expiresAt - MANDATORY_REFRESH_THRESHOLD_IN_SECONDS)) {
      return { token: accessToken, expiresAt: expiresAt ?? null };
    }
    const refreshToken = creds.refresh_token;
    if (!config.clientId || !refreshToken) {
      throw new WorkloadIdentityError(`Access token at ${config.credentialsPath} has expired and no refresh is available (client_id ${config.clientId ? "set" : "empty"}, refresh_token ${refreshToken ? "set" : "empty"})`);
    }
    requireSecureTokenEndpoint(config.baseURL);
    const body = {
      grant_type: GRANT_TYPE_REFRESH_TOKEN,
      refresh_token: refreshToken,
      client_id: config.clientId
    };
    const url = `${config.baseURL}${TOKEN_ENDPOINT}`;
    let resp;
    try {
      resp = await config.fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "anthropic-beta": OAUTH_API_BETA_HEADER,
          "User-Agent": config.userAgent || `anthropic-sdk-typescript/${VERSION} userOAuthProvider`
        },
        body: JSON.stringify(body)
      });
    } catch (err) {
      throw new WorkloadIdentityError(`User OAuth refresh failed to reach token endpoint: ${err}`);
    }
    const requestId = resp.headers.get("Request-Id");
    if (!resp.ok) {
      const text = await resp.text().catch(() => "");
      throw new WorkloadIdentityError(`User OAuth refresh failed (HTTP ${resp.status}): ${redactSensitive(text)}`, resp.status, redactSensitive(text), requestId);
    }
    const data = await parseTokenResponse(resp, requestId);
    const expiresIn = Number(data.expires_in);
    if (!Number.isFinite(expiresIn)) {
      throw new WorkloadIdentityError(`User OAuth refresh response missing or invalid expires_in: ${JSON.stringify(redactSensitive(data))}`, resp.status, redactSensitive(data), requestId);
    }
    const newExpiresAt = nowAsSeconds() + expiresIn;
    const newRefreshToken = data.refresh_token || refreshToken;
    await writeCredentialsFileAtomic(config.credentialsPath, {
      ...creds,
      version: CREDENTIALS_FILE_VERSION,
      type: "oauth_token",
      access_token: data.access_token,
      expires_at: newExpiresAt,
      refresh_token: newRefreshToken
    });
    return { token: data.access_token, expiresAt: newExpiresAt };
  };
}
var init_user_oauth = __esm({
  "node_modules/@anthropic-ai/sdk/lib/credentials/user-oauth.mjs"() {
    init_credentials();
    init_types();
    init_time();
    init_version();
  }
});

// node_modules/@anthropic-ai/sdk/lib/credentials/credential-chain.mjs
function resolveCredentialsFromConfig(config, options) {
  const credentialsPath = config.authentication.credentials_path ?? null;
  const effectiveBaseURL = (config.base_url || options.baseURL).replace(/\/+$/, "");
  const provider2 = buildProvider(config, credentialsPath, effectiveBaseURL, options);
  const extraHeaders = {};
  if (config.workspace_id && config.authentication.type === "user_oauth") {
    extraHeaders["anthropic-workspace-id"] = config.workspace_id;
  }
  return { provider: provider2, extraHeaders, baseURL: config.base_url || void 0 };
}
async function defaultCredentials(options, profile) {
  const loaded = await loadConfigWithSource(profile);
  if (!loaded) {
    return null;
  }
  const { config, fromFile } = loaded;
  const withPath = config.authentication.credentials_path || !fromFile ? config : {
    ...config,
    authentication: {
      ...config.authentication,
      credentials_path: await getCredentialsPath(config, profile) ?? void 0
    }
  };
  return resolveCredentialsFromConfig(withPath, options);
}
function buildProvider(config, credentialsPath, baseURL, options) {
  switch (config.authentication.type) {
    case "oidc_federation": {
      const auth = config.authentication;
      const identityProvider = resolveIdentityTokenProvider(auth);
      if (!identityProvider) {
        throw new WorkloadIdentityError("oidc_federation config requires an identity token (set authentication.identity_token, ANTHROPIC_IDENTITY_TOKEN_FILE, or ANTHROPIC_IDENTITY_TOKEN)");
      }
      if (!auth.federation_rule_id) {
        throw new WorkloadIdentityError("oidc_federation config requires 'federation_rule_id'. Set it in authentication.federation_rule_id in your profile, or via ANTHROPIC_FEDERATION_RULE_ID (profile takes precedence).");
      }
      if (!config.organization_id) {
        throw new WorkloadIdentityError("oidc_federation config requires organization_id (set ANTHROPIC_ORGANIZATION_ID or config.organization_id)");
      }
      const exchange = oidcFederationProvider({
        identityTokenProvider: identityProvider,
        federationRuleId: auth.federation_rule_id,
        organizationId: config.organization_id,
        serviceAccountId: auth.service_account_id,
        workspaceId: config.workspace_id,
        baseURL,
        fetch: options.fetch,
        userAgent: options.userAgent
      });
      if (credentialsPath) {
        return cachedExchangeProvider(exchange, credentialsPath, options.onCacheWriteError, options.onSafetyWarning);
      }
      return exchange;
    }
    case "user_oauth": {
      if (!credentialsPath) {
        throw new WorkloadIdentityError("user_oauth config requires authentication.credentials_path (or load via a profile so it defaults to <config_dir>/credentials/<profile>.json)");
      }
      return userOAuthProvider({
        credentialsPath,
        clientId: config.authentication.client_id,
        baseURL,
        fetch: options.fetch,
        userAgent: options.userAgent,
        onSafetyWarning: options.onSafetyWarning
      });
    }
    default: {
      const t = config.authentication.type;
      throw new WorkloadIdentityError(`authentication.type "${t}" is not a known authentication type`);
    }
  }
}
function resolveIdentityTokenProvider(auth) {
  if (auth.identity_token) {
    const source = auth.identity_token.source;
    if (source !== "file") {
      throw new WorkloadIdentityError(`identity_token.source "${source}" is not supported by this SDK version (only "file")`);
    }
    if (!auth.identity_token.path) {
      throw new WorkloadIdentityError(`identity_token.source "file" requires a non-empty path`);
    }
    return identityTokenFromFile(auth.identity_token.path);
  }
  const tokenFile = readEnv("ANTHROPIC_IDENTITY_TOKEN_FILE");
  if (tokenFile) {
    return identityTokenFromFile(tokenFile);
  }
  const tokenValue = readEnv("ANTHROPIC_IDENTITY_TOKEN");
  if (tokenValue) {
    return identityTokenFromValue(tokenValue);
  }
  return null;
}
function cachedExchangeProvider(exchange, credentialsPath, onCacheWriteError, onSafetyWarning) {
  return async (opts) => {
    const fs5 = await import("node:fs");
    await checkCredentialsFileSafety(credentialsPath, onSafetyWarning);
    let existing;
    try {
      const raw = await fs5.promises.readFile(credentialsPath, "utf-8");
      existing = JSON.parse(raw);
      const token2 = existing?.["access_token"];
      if (token2 && !opts?.forceRefresh) {
        const expiresAt = existing?.["expires_at"];
        if (expiresAt == null || nowAsSeconds() < expiresAt - MANDATORY_REFRESH_THRESHOLD_IN_SECONDS) {
          return { token: token2, expiresAt: expiresAt ?? null };
        }
      }
    } catch (err) {
      const code = err?.code;
      if (code !== "ENOENT" && !(err instanceof SyntaxError)) {
        onCacheWriteError?.(err);
      }
    }
    const result = await exchange(opts);
    try {
      await writeCredentialsFileAtomic(credentialsPath, {
        ...existing ?? {},
        version: CREDENTIALS_FILE_VERSION,
        type: "oauth_token",
        access_token: result.token,
        expires_at: result.expiresAt
      });
    } catch (err) {
      onCacheWriteError?.(err);
    }
    return result;
  };
}
var init_credential_chain = __esm({
  "node_modules/@anthropic-ai/sdk/lib/credentials/credential-chain.mjs"() {
    init_env();
    init_credentials();
    init_types();
    init_time();
    init_identity_token();
    init_oidc_federation();
    init_user_oauth();
  }
});

// node_modules/@anthropic-ai/sdk/internal/decoders/line.mjs
function findNewlineIndex(buffer, startIndex) {
  const newline = 10;
  const carriage = 13;
  for (let i = startIndex ?? 0; i < buffer.length; i++) {
    if (buffer[i] === newline) {
      return { preceding: i, index: i + 1, carriage: false };
    }
    if (buffer[i] === carriage) {
      return { preceding: i, index: i + 1, carriage: true };
    }
  }
  return null;
}
function findDoubleNewlineIndex(buffer) {
  const newline = 10;
  const carriage = 13;
  for (let i = 0; i < buffer.length - 1; i++) {
    if (buffer[i] === newline && buffer[i + 1] === newline) {
      return i + 2;
    }
    if (buffer[i] === carriage && buffer[i + 1] === carriage) {
      return i + 2;
    }
    if (buffer[i] === carriage && buffer[i + 1] === newline && i + 3 < buffer.length && buffer[i + 2] === carriage && buffer[i + 3] === newline) {
      return i + 4;
    }
  }
  return -1;
}
var _LineDecoder_buffer, _LineDecoder_carriageReturnIndex, LineDecoder;
var init_line = __esm({
  "node_modules/@anthropic-ai/sdk/internal/decoders/line.mjs"() {
    init_tslib();
    init_bytes();
    LineDecoder = class {
      constructor() {
        _LineDecoder_buffer.set(this, void 0);
        _LineDecoder_carriageReturnIndex.set(this, void 0);
        __classPrivateFieldSet(this, _LineDecoder_buffer, new Uint8Array(), "f");
        __classPrivateFieldSet(this, _LineDecoder_carriageReturnIndex, null, "f");
      }
      decode(chunk) {
        if (chunk == null) {
          return [];
        }
        const binaryChunk = chunk instanceof ArrayBuffer ? new Uint8Array(chunk) : typeof chunk === "string" ? encodeUTF8(chunk) : chunk;
        __classPrivateFieldSet(this, _LineDecoder_buffer, concatBytes([__classPrivateFieldGet(this, _LineDecoder_buffer, "f"), binaryChunk]), "f");
        const lines = [];
        let patternIndex;
        while ((patternIndex = findNewlineIndex(__classPrivateFieldGet(this, _LineDecoder_buffer, "f"), __classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f"))) != null) {
          if (patternIndex.carriage && __classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f") == null) {
            __classPrivateFieldSet(this, _LineDecoder_carriageReturnIndex, patternIndex.index, "f");
            continue;
          }
          if (__classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f") != null && (patternIndex.index !== __classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f") + 1 || patternIndex.carriage)) {
            lines.push(decodeUTF8(__classPrivateFieldGet(this, _LineDecoder_buffer, "f").subarray(0, __classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f") - 1)));
            __classPrivateFieldSet(this, _LineDecoder_buffer, __classPrivateFieldGet(this, _LineDecoder_buffer, "f").subarray(__classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f")), "f");
            __classPrivateFieldSet(this, _LineDecoder_carriageReturnIndex, null, "f");
            continue;
          }
          const endIndex = __classPrivateFieldGet(this, _LineDecoder_carriageReturnIndex, "f") !== null ? patternIndex.preceding - 1 : patternIndex.preceding;
          const line = decodeUTF8(__classPrivateFieldGet(this, _LineDecoder_buffer, "f").subarray(0, endIndex));
          lines.push(line);
          __classPrivateFieldSet(this, _LineDecoder_buffer, __classPrivateFieldGet(this, _LineDecoder_buffer, "f").subarray(patternIndex.index), "f");
          __classPrivateFieldSet(this, _LineDecoder_carriageReturnIndex, null, "f");
        }
        return lines;
      }
      flush() {
        if (!__classPrivateFieldGet(this, _LineDecoder_buffer, "f").length) {
          return [];
        }
        return this.decode("\n");
      }
    };
    _LineDecoder_buffer = /* @__PURE__ */ new WeakMap(), _LineDecoder_carriageReturnIndex = /* @__PURE__ */ new WeakMap();
    LineDecoder.NEWLINE_CHARS = /* @__PURE__ */ new Set(["\n", "\r"]);
    LineDecoder.NEWLINE_REGEXP = /\r\n|[\n\r]/g;
  }
});

// node_modules/@anthropic-ai/sdk/core/streaming.mjs
async function* _iterSSEMessages(response, controller) {
  if (!response.body) {
    controller.abort();
    if (typeof globalThis.navigator !== "undefined" && globalThis.navigator.product === "ReactNative") {
      throw new AnthropicError(`The default react-native fetch implementation does not support streaming. Please use expo/fetch: https://docs.expo.dev/versions/latest/sdk/expo/#expofetch-api`);
    }
    throw new AnthropicError(`Attempted to iterate over a response with no body`);
  }
  const sseDecoder = new SSEDecoder();
  const lineDecoder = new LineDecoder();
  const iter = ReadableStreamToAsyncIterable(response.body);
  for await (const sseChunk of iterSSEChunks(iter)) {
    for (const line of lineDecoder.decode(sseChunk)) {
      const sse = sseDecoder.decode(line);
      if (sse)
        yield sse;
    }
  }
  for (const line of lineDecoder.flush()) {
    const sse = sseDecoder.decode(line);
    if (sse)
      yield sse;
  }
}
async function* iterSSEChunks(iterator) {
  let data = new Uint8Array();
  for await (const chunk of iterator) {
    if (chunk == null) {
      continue;
    }
    const binaryChunk = chunk instanceof ArrayBuffer ? new Uint8Array(chunk) : typeof chunk === "string" ? encodeUTF8(chunk) : chunk;
    let newData = new Uint8Array(data.length + binaryChunk.length);
    newData.set(data);
    newData.set(binaryChunk, data.length);
    data = newData;
    let patternIndex;
    while ((patternIndex = findDoubleNewlineIndex(data)) !== -1) {
      yield data.slice(0, patternIndex);
      data = data.slice(patternIndex);
    }
  }
  if (data.length > 0) {
    yield data;
  }
}
function partition(str, delimiter2) {
  const index = str.indexOf(delimiter2);
  if (index !== -1) {
    return [str.substring(0, index), delimiter2, str.substring(index + delimiter2.length)];
  }
  return [str, "", ""];
}
var _Stream_client, Stream, SSEDecoder;
var init_streaming = __esm({
  "node_modules/@anthropic-ai/sdk/core/streaming.mjs"() {
    init_tslib();
    init_error();
    init_shims();
    init_line();
    init_shims();
    init_errors();
    init_values();
    init_bytes();
    init_log();
    init_error();
    init_request_signal();
    Stream = class _Stream {
      constructor(iterator, controller, client) {
        this.iterator = iterator;
        _Stream_client.set(this, void 0);
        this.controller = controller;
        __classPrivateFieldSet(this, _Stream_client, client, "f");
      }
      /**
       * Iterate the raw Server-Sent Events from `response` — `{event, data, raw}`
       * objects, before any JSON parsing or event-name filtering.
       *
       * This reads `response.body` directly (not a clone), so the response is
       * consumed. Use this in middleware that fully replaces the stream body; for
       * read-only observation of parsed events, use `ctx.parse()` instead.
       */
      static rawEvents(response, controller = new AbortController()) {
        return _iterSSEMessages(response, controller);
      }
      static fromSSEResponse(response, controller, client) {
        let consumed = false;
        const logger = client ? loggerFor(client) : console;
        async function* iterator() {
          if (consumed) {
            throw new AnthropicError("Cannot iterate over a consumed stream, use `.tee()` to split the stream.");
          }
          consumed = true;
          let done = false;
          try {
            for await (const sse of _iterSSEMessages(response, controller)) {
              if (sse.event === "completion") {
                try {
                  yield JSON.parse(sse.data);
                } catch (e) {
                  logger.error(`Could not parse message into JSON:`, sse.data);
                  logger.error(`From chunk:`, sse.raw);
                  throw e;
                }
              }
              if (sse.event === "message_start" || sse.event === "message_delta" || sse.event === "message_stop" || sse.event === "content_block_start" || sse.event === "content_block_delta" || sse.event === "content_block_stop" || sse.event === "message" || sse.event === "user.message" || sse.event === "user.interrupt" || sse.event === "user.tool_confirmation" || sse.event === "user.custom_tool_result" || sse.event === "user.tool_result" || sse.event === "agent.message" || sse.event === "agent.thinking" || sse.event === "agent.tool_use" || sse.event === "agent.tool_result" || sse.event === "agent.mcp_tool_use" || sse.event === "agent.mcp_tool_result" || sse.event === "agent.custom_tool_use" || sse.event === "agent.thread_context_compacted" || sse.event === "session.status_running" || sse.event === "session.status_idle" || sse.event === "session.status_rescheduled" || sse.event === "session.status_terminated" || sse.event === "session.error" || sse.event === "session.deleted" || sse.event === "session.updated" || sse.event === "span.model_request_start" || sse.event === "span.model_request_end" || sse.event === "span.outcome_evaluation_start" || sse.event === "span.outcome_evaluation_ongoing" || sse.event === "span.outcome_evaluation_end" || sse.event === "user.define_outcome" || sse.event === "agent.thread_message_received" || sse.event === "agent.thread_message_sent" || sse.event === "agent.session_thread_message_received" || sse.event === "agent.session_thread_message_sent" || sse.event === "session.thread_created" || sse.event === "session.thread_status_created" || sse.event === "session.thread_status_running" || sse.event === "session.thread_status_idle" || sse.event === "session.thread_status_rescheduled" || sse.event === "session.thread_status_terminated" || sse.event === "event_start" || sse.event === "event_delta" || sse.event === "system.message") {
                try {
                  yield JSON.parse(sse.data);
                } catch (e) {
                  logger.error(`Could not parse message into JSON:`, sse.data);
                  logger.error(`From chunk:`, sse.raw);
                  throw e;
                }
              }
              if (sse.event === "ping") {
                continue;
              }
              if (sse.event === "error") {
                const body = safeJSON(sse.data) ?? sse.data;
                const type = body?.error?.type;
                throw new APIError(void 0, body, void 0, response.headers, type);
              }
            }
            done = true;
          } catch (e) {
            if (isAbortError(e))
              return;
            throw e;
          } finally {
            if (!done)
              controller.abort();
            releaseRequestSignal(controller);
          }
        }
        return new _Stream(iterator, controller, client);
      }
      /**
       * Generates a Stream from a newline-separated ReadableStream
       * where each item is a JSON value.
       */
      static fromReadableStream(readableStream, controller, client) {
        let consumed = false;
        async function* iterLines() {
          const lineDecoder = new LineDecoder();
          const iter = ReadableStreamToAsyncIterable(readableStream);
          for await (const chunk of iter) {
            for (const line of lineDecoder.decode(chunk)) {
              yield line;
            }
          }
          for (const line of lineDecoder.flush()) {
            yield line;
          }
        }
        async function* iterator() {
          if (consumed) {
            throw new AnthropicError("Cannot iterate over a consumed stream, use `.tee()` to split the stream.");
          }
          consumed = true;
          let done = false;
          try {
            for await (const line of iterLines()) {
              if (done)
                continue;
              if (line)
                yield JSON.parse(line);
            }
            done = true;
          } catch (e) {
            if (isAbortError(e))
              return;
            throw e;
          } finally {
            if (!done)
              controller.abort();
            releaseRequestSignal(controller);
          }
        }
        return new _Stream(iterator, controller, client);
      }
      [(_Stream_client = /* @__PURE__ */ new WeakMap(), Symbol.asyncIterator)]() {
        return this.iterator();
      }
      /**
       * Splits the stream into two streams which can be
       * independently read from at different speeds.
       */
      tee() {
        const left = [];
        const right = [];
        const iterator = this.iterator();
        const teeIterator = (queue) => {
          return {
            next: () => {
              if (queue.length === 0) {
                const result = iterator.next();
                left.push(result);
                right.push(result);
              }
              return queue.shift();
            }
          };
        };
        return [
          new _Stream(() => teeIterator(left), this.controller, __classPrivateFieldGet(this, _Stream_client, "f")),
          new _Stream(() => teeIterator(right), this.controller, __classPrivateFieldGet(this, _Stream_client, "f"))
        ];
      }
      /**
       * Converts this stream to a newline-separated ReadableStream of
       * JSON stringified values in the stream
       * which can be turned back into a Stream with `Stream.fromReadableStream()`.
       */
      toReadableStream() {
        const self = this;
        let iter;
        return makeReadableStream({
          async start() {
            iter = self[Symbol.asyncIterator]();
          },
          async pull(ctrl) {
            try {
              const { value, done } = await iter.next();
              if (done)
                return ctrl.close();
              const bytes = encodeUTF8(JSON.stringify(value) + "\n");
              ctrl.enqueue(bytes);
            } catch (err) {
              ctrl.error(err);
            }
          },
          async cancel() {
            await iter.return?.();
          }
        });
      }
    };
    SSEDecoder = class {
      constructor() {
        this.event = null;
        this.data = [];
        this.chunks = [];
      }
      decode(line) {
        if (line.endsWith("\r")) {
          line = line.substring(0, line.length - 1);
        }
        if (!line) {
          if (!this.event && !this.data.length)
            return null;
          const sse = {
            event: this.event,
            data: this.data.join("\n"),
            raw: this.chunks
          };
          this.event = null;
          this.data = [];
          this.chunks = [];
          return sse;
        }
        this.chunks.push(line);
        if (line.startsWith(":")) {
          return null;
        }
        let [fieldname, _, value] = partition(line, ":");
        if (value.startsWith(" ")) {
          value = value.substring(1);
        }
        if (fieldname === "event") {
          this.event = value;
        } else if (fieldname === "data") {
          this.data.push(value);
        }
        return null;
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/parse.mjs
async function defaultParseResponse(client, props) {
  const { response, requestLogID, retryOfRequestLogID, startTime } = props;
  const body = await (async () => {
    if (props.options.stream) {
      loggerFor(client).debug("response", response.status, response.url, response.headers, response.body);
      return Stream.fromSSEResponse(response, props.controller);
    }
    if (response.status === 204) {
      return null;
    }
    if (props.options.__binaryResponse) {
      return response;
    }
    const contentType = response.headers.get("content-type");
    const mediaType = contentType?.split(";")[0]?.trim();
    const isJSON = mediaType?.includes("application/json") || mediaType?.endsWith("+json");
    if (isJSON) {
      const contentLength = response.headers.get("content-length");
      if (contentLength === "0") {
        return void 0;
      }
      const json = await response.json();
      return addRequestID(json, response);
    }
    const text = await response.text();
    return text;
  })().finally(() => {
    if (!props.options.stream && !props.options.__binaryResponse) {
      releaseRequestSignal(props.controller);
    }
  });
  loggerFor(client).debug(`[${requestLogID}] response parsed`, formatRequestDetails({
    retryOfRequestLogID,
    url: response.url,
    status: response.status,
    body,
    durationMs: Date.now() - startTime
  }));
  return body;
}
function addRequestID(value, response) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return value;
  }
  return Object.defineProperty(value, "_request_id", {
    value: response.headers.get("request-id"),
    enumerable: false
  });
}
var init_parse = __esm({
  "node_modules/@anthropic-ai/sdk/internal/parse.mjs"() {
    init_streaming();
    init_log();
    init_request_signal();
  }
});

// node_modules/@anthropic-ai/sdk/core/middleware.mjs
function isFetchOriginError(err) {
  return typeof err === "object" && err !== null && fetchOriginErrors.has(err);
}
function isRetryableError(err) {
  const seen = /* @__PURE__ */ new Set();
  while (typeof err === "object" && err !== null && !seen.has(err)) {
    seen.add(err);
    if (isFetchOriginError(err) || isAbortError(err) || err instanceof APIConnectionError || err instanceof RetryableError) {
      return true;
    }
    err = err.cause;
  }
  return false;
}
function wrapFetchWithMiddleware(fetchFn, middleware, options, client) {
  return async (url, init = {}) => {
    if (middleware.length === 0) {
      return fetchFn.call(void 0, url, init);
    }
    const headers = init.headers instanceof Headers ? init.headers : new Headers(init.headers);
    const response = await applyMiddleware(fetchFn, middleware, options, client)({
      ...init,
      headers,
      url: typeof url === "string" ? url : url instanceof URL ? url.href : url.url
    });
    if (response.bodyUsed || response.body?.locked) {
      throw new AnthropicError("middleware consumed the response body; use response.clone() to inspect it, or return new Response(body, response) to consume and replace it");
    }
    return response;
  };
}
function createMiddlewareContext(options, client) {
  const cache = /* @__PURE__ */ new WeakMap();
  return {
    options,
    // Resolved per chain, so changes to the client's `logLevel`/`logger`
    // apply to subsequent requests.
    logger: client ? loggerFor(client) : defaultLogger(),
    parse(response) {
      if (options?.stream && response.ok) {
        return parseMiddlewareResponse(response, options);
      }
      let parsed = cache.get(response);
      if (!parsed) {
        parsed = parseMiddlewareResponse(response, options);
        cache.set(response, parsed);
      }
      return parsed;
    }
  };
}
async function parseMiddlewareResponse(response, options) {
  if (response.bodyUsed || response.body?.locked) {
    throw new AnthropicError("cannot ctx.parse() a response whose body was already consumed; call ctx.parse() instead of reading the body, or read via response.clone()");
  }
  if (options?.stream && response.ok) {
    return Stream.fromSSEResponse(response.clone(), new AbortController());
  }
  if (response.status === 204) {
    return null;
  }
  if (options?.__binaryResponse) {
    return response;
  }
  const contentType = response.headers.get("content-type");
  const mediaType = contentType?.split(";")[0]?.trim();
  const isJSON = mediaType?.includes("application/json") || mediaType?.endsWith("+json");
  if (isJSON) {
    if (response.headers.get("content-length") === "0") {
      return void 0;
    }
    return addRequestID(await response.clone().json(), response);
  }
  return await response.clone().text();
}
function applyMiddleware(fetchFn, middleware, options, client) {
  let next = async ({ url, ...init }) => {
    try {
      return await fetchFn.call(void 0, url, init);
    } catch (err) {
      const error = castToError(err);
      fetchOriginErrors.add(error);
      throw error;
    }
  };
  const ctx = createMiddlewareContext(options, client);
  for (let i = middleware.length - 1; i >= 0; i--) {
    const mw = middleware[i];
    const nextInner = next;
    next = async (request) => mw(request, nextInner, ctx);
  }
  return next;
}
var fetchOriginErrors;
var init_middleware = __esm({
  "node_modules/@anthropic-ai/sdk/core/middleware.mjs"() {
    init_errors();
    init_parse();
    init_log();
    init_error();
    init_streaming();
    fetchOriginErrors = /* @__PURE__ */ new WeakSet();
  }
});

// node_modules/@anthropic-ai/sdk/core/api-promise.mjs
var _APIPromise_client, APIPromise;
var init_api_promise = __esm({
  "node_modules/@anthropic-ai/sdk/core/api-promise.mjs"() {
    init_tslib();
    init_parse();
    APIPromise = class _APIPromise extends Promise {
      constructor(client, responsePromise, parseResponse = defaultParseResponse) {
        super((resolve4) => {
          resolve4(null);
        });
        this.responsePromise = responsePromise;
        this.parseResponse = parseResponse;
        _APIPromise_client.set(this, void 0);
        __classPrivateFieldSet(this, _APIPromise_client, client, "f");
      }
      _thenUnwrap(transform) {
        return new _APIPromise(__classPrivateFieldGet(this, _APIPromise_client, "f"), this.responsePromise, async (client, props) => addRequestID(transform(await this.parseResponse(client, props), props), props.response));
      }
      /**
       * Gets the raw `Response` instance instead of parsing the response
       * data.
       *
       * If you want to parse the response body but still get the `Response`
       * instance, you can use {@link withResponse()}.
       *
       * 👋 Getting the wrong TypeScript type for `Response`?
       * Try setting `"moduleResolution": "NodeNext"` or add `"lib": ["DOM"]`
       * to your `tsconfig.json`.
       */
      asResponse() {
        return this.responsePromise.then((p) => p.response);
      }
      /**
       * Gets the parsed response data, the raw `Response` instance and the ID of the request,
       * returned via the `request-id` header which is useful for debugging requests and resporting
       * issues to Anthropic.
       *
       * If you just want to get the raw `Response` instance without parsing it,
       * you can use {@link asResponse()}.
       *
       * 👋 Getting the wrong TypeScript type for `Response`?
       * Try setting `"moduleResolution": "NodeNext"` or add `"lib": ["DOM"]`
       * to your `tsconfig.json`.
       */
      async withResponse() {
        const [data, response] = await Promise.all([this.parse(), this.asResponse()]);
        return { data, response, request_id: response.headers.get("request-id") };
      }
      parse() {
        if (!this.parsedPromise) {
          this.parsedPromise = this.responsePromise.then((data) => this.parseResponse(__classPrivateFieldGet(this, _APIPromise_client, "f"), data));
        }
        return this.parsedPromise;
      }
      then(onfulfilled, onrejected) {
        return this.parse().then(onfulfilled, onrejected);
      }
      catch(onrejected) {
        return this.parse().catch(onrejected);
      }
      finally(onfinally) {
        return this.parse().finally(onfinally);
      }
    };
    _APIPromise_client = /* @__PURE__ */ new WeakMap();
  }
});

// node_modules/@anthropic-ai/sdk/core/pagination.mjs
var _AbstractPage_client, AbstractPage, PagePromise, Page, PageCursor, BidirectionalPageCursor;
var init_pagination = __esm({
  "node_modules/@anthropic-ai/sdk/core/pagination.mjs"() {
    init_tslib();
    init_error();
    init_parse();
    init_api_promise();
    init_values();
    AbstractPage = class {
      constructor(client, response, body, options) {
        _AbstractPage_client.set(this, void 0);
        __classPrivateFieldSet(this, _AbstractPage_client, client, "f");
        this.options = options;
        this.response = response;
        this.body = body;
      }
      hasNextPage() {
        const items = this.getPaginatedItems();
        if (!items.length)
          return false;
        return this.nextPageRequestOptions() != null;
      }
      async getNextPage() {
        const nextOptions = this.nextPageRequestOptions();
        if (!nextOptions) {
          throw new AnthropicError("No next page expected; please check `.hasNextPage()` before calling `.getNextPage()`.");
        }
        return await __classPrivateFieldGet(this, _AbstractPage_client, "f").requestAPIList(this.constructor, nextOptions);
      }
      async *iterPages() {
        let page = this;
        yield page;
        while (page.hasNextPage()) {
          page = await page.getNextPage();
          yield page;
        }
      }
      async *[(_AbstractPage_client = /* @__PURE__ */ new WeakMap(), Symbol.asyncIterator)]() {
        for await (const page of this.iterPages()) {
          for (const item of page.getPaginatedItems()) {
            yield item;
          }
        }
      }
    };
    PagePromise = class extends APIPromise {
      constructor(client, request, Page2) {
        super(client, request, async (client2, props) => new Page2(client2, props.response, await defaultParseResponse(client2, props), props.options));
      }
      /**
       * Allow auto-paginating iteration on an unawaited list call, eg:
       *
       *    for await (const item of client.items.list()) {
       *      console.log(item)
       *    }
       */
      async *[Symbol.asyncIterator]() {
        const page = await this;
        for await (const item of page) {
          yield item;
        }
      }
    };
    Page = class extends AbstractPage {
      constructor(client, response, body, options) {
        super(client, response, body, options);
        this.data = body.data || [];
        this.has_more = body.has_more || false;
        this.first_id = body.first_id || null;
        this.last_id = body.last_id || null;
      }
      getPaginatedItems() {
        return this.data ?? [];
      }
      hasNextPage() {
        if (this.has_more === false) {
          return false;
        }
        return super.hasNextPage();
      }
      nextPageRequestOptions() {
        if (this.options.query?.["before_id"]) {
          const first_id = this.first_id;
          if (!first_id) {
            return null;
          }
          return {
            ...this.options,
            query: {
              ...maybeObj(this.options.query),
              before_id: first_id
            }
          };
        }
        const cursor = this.last_id;
        if (!cursor) {
          return null;
        }
        return {
          ...this.options,
          query: {
            ...maybeObj(this.options.query),
            after_id: cursor
          }
        };
      }
    };
    PageCursor = class extends AbstractPage {
      constructor(client, response, body, options) {
        super(client, response, body, options);
        this.data = body.data || [];
        this.next_page = body.next_page || null;
      }
      getPaginatedItems() {
        return this.data ?? [];
      }
      nextPageRequestOptions() {
        const cursor = this.next_page;
        if (!cursor) {
          return null;
        }
        return {
          ...this.options,
          query: {
            ...maybeObj(this.options.query),
            page: cursor
          }
        };
      }
    };
    BidirectionalPageCursor = class extends AbstractPage {
      constructor(client, response, body, options) {
        super(client, response, body, options);
        this.data = body.data || [];
        this.next_page = body.next_page || null;
        this.prev_page = body.prev_page || null;
      }
      getPaginatedItems() {
        return this.data ?? [];
      }
      nextPageRequestOptions() {
        const cursor = this.next_page;
        if (!cursor) {
          return null;
        }
        return {
          ...this.options,
          query: {
            ...maybeObj(this.options.query),
            page: cursor
          }
        };
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/uploads.mjs
function makeFile(fileBits, fileName, options) {
  checkFileSupport();
  return new File(fileBits, fileName ?? "unknown_file", options);
}
function getName(value, stripPath) {
  const val = typeof value === "object" && value !== null && ("name" in value && value.name && String(value.name) || "url" in value && value.url && String(value.url) || "filename" in value && value.filename && String(value.filename) || "path" in value && value.path && String(value.path)) || "";
  return stripPath ? val.split(/[\\/]/).pop() || void 0 : val;
}
function supportsFormData(fetchObject) {
  const fetch2 = typeof fetchObject === "function" ? fetchObject : fetchObject.fetch;
  const cached = supportsFormDataMap.get(fetch2);
  if (cached)
    return cached;
  const promise = (async () => {
    try {
      const FetchResponse = "Response" in fetch2 ? fetch2.Response : (await fetch2("data:,")).constructor;
      const data = new FormData();
      if (data.toString() === await new FetchResponse(data).text()) {
        return false;
      }
      return true;
    } catch {
      return true;
    }
  })();
  supportsFormDataMap.set(fetch2, promise);
  return promise;
}
var checkFileSupport, isAsyncIterable, multipartFormRequestOptions, supportsFormDataMap, createForm, isNamedBlob, addFormValue;
var init_uploads = __esm({
  "node_modules/@anthropic-ai/sdk/internal/uploads.mjs"() {
    init_shims();
    checkFileSupport = () => {
      if (typeof File === "undefined") {
        const { process: process2 } = globalThis;
        const isOldNode = typeof process2?.versions?.node === "string" && parseInt(process2.versions.node.split(".")) < 20;
        throw new Error("`File` is not defined as a global, which is required for file uploads." + (isOldNode ? " Update to Node 20 LTS or newer, or set `globalThis.File` to `import('node:buffer').File`." : ""));
      }
    };
    isAsyncIterable = (value) => value != null && typeof value === "object" && typeof value[Symbol.asyncIterator] === "function";
    multipartFormRequestOptions = async (opts, fetch2, stripFilenames = true) => {
      return { ...opts, body: await createForm(opts.body, fetch2, stripFilenames) };
    };
    supportsFormDataMap = /* @__PURE__ */ new WeakMap();
    createForm = async (body, fetch2, stripFilenames = true) => {
      if (!await supportsFormData(fetch2)) {
        throw new TypeError("The provided fetch function does not support file uploads with the current global FormData class.");
      }
      const form = new FormData();
      await Promise.all(Object.entries(body || {}).map(([key, value]) => addFormValue(form, key, value, stripFilenames)));
      return form;
    };
    isNamedBlob = (value) => value instanceof Blob && "name" in value;
    addFormValue = async (form, key, value, stripFilenames) => {
      if (value === void 0)
        return;
      if (value == null) {
        throw new TypeError(`Received null for "${key}"; to pass null in FormData, you must use the string 'null'`);
      }
      if (typeof value === "string" || typeof value === "number" || typeof value === "boolean") {
        form.append(key, String(value));
      } else if (value instanceof Response) {
        let options = {};
        const contentType = value.headers.get("Content-Type");
        if (contentType) {
          options = { type: contentType };
        }
        form.append(key, makeFile([await value.blob()], getName(value, stripFilenames), options));
      } else if (isAsyncIterable(value)) {
        form.append(key, makeFile([await new Response(ReadableStreamFrom(value)).blob()], getName(value, stripFilenames)));
      } else if (isNamedBlob(value)) {
        form.append(key, makeFile([value], getName(value, stripFilenames), { type: value.type }));
      } else if (Array.isArray(value)) {
        await Promise.all(value.map((entry) => addFormValue(form, key + "[]", entry, stripFilenames)));
      } else if (typeof value === "object") {
        await Promise.all(Object.entries(value).map(([name, prop]) => addFormValue(form, `${key}[${name}]`, prop, stripFilenames)));
      } else {
        throw new TypeError(`Invalid value given to form, expected a string, number, boolean, object, Array, File or Blob but got ${value} instead`);
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/to-file.mjs
async function toFile(value, name, options) {
  checkFileSupport();
  value = await value;
  name || (name = getName(value, true));
  if (isFileLike(value)) {
    if (value instanceof File && name == null && options == null) {
      return value;
    }
    return makeFile([await value.arrayBuffer()], name ?? value.name, {
      type: value.type,
      lastModified: value.lastModified,
      ...options
    });
  }
  if (isResponseLike(value)) {
    const blob = await value.blob();
    name || (name = new URL(value.url).pathname.split(/[\\/]/).pop());
    return makeFile(await getBytes(blob), name, options);
  }
  const parts = await getBytes(value);
  if (!options?.type) {
    const type = parts.find((part) => typeof part === "object" && "type" in part && part.type);
    if (typeof type === "string") {
      options = { ...options, type };
    }
  }
  return makeFile(parts, name, options);
}
async function getBytes(value) {
  let parts = [];
  if (typeof value === "string" || ArrayBuffer.isView(value) || // includes Uint8Array, Buffer, etc.
  value instanceof ArrayBuffer) {
    parts.push(value);
  } else if (isBlobLike(value)) {
    parts.push(value instanceof Blob ? value : await value.arrayBuffer());
  } else if (isAsyncIterable(value)) {
    for await (const chunk of value) {
      parts.push(...await getBytes(chunk));
    }
  } else {
    const constructor = value?.constructor?.name;
    throw new Error(`Unexpected data type: ${typeof value}${constructor ? `; constructor: ${constructor}` : ""}${propsForError(value)}`);
  }
  return parts;
}
function propsForError(value) {
  if (typeof value !== "object" || value === null)
    return "";
  const props = Object.getOwnPropertyNames(value);
  return `; props: [${props.map((p) => `"${p}"`).join(", ")}]`;
}
var isBlobLike, isFileLike, isResponseLike;
var init_to_file = __esm({
  "node_modules/@anthropic-ai/sdk/internal/to-file.mjs"() {
    init_uploads();
    init_uploads();
    isBlobLike = (value) => value != null && typeof value === "object" && typeof value.size === "number" && typeof value.type === "string" && typeof value.text === "function" && typeof value.slice === "function" && typeof value.arrayBuffer === "function";
    isFileLike = (value) => value != null && typeof value === "object" && typeof value.name === "string" && typeof value.lastModified === "number" && isBlobLike(value);
    isResponseLike = (value) => value != null && typeof value === "object" && typeof value.url === "string" && typeof value.blob === "function";
  }
});

// node_modules/@anthropic-ai/sdk/core/uploads.mjs
var init_uploads2 = __esm({
  "node_modules/@anthropic-ai/sdk/core/uploads.mjs"() {
    init_to_file();
  }
});

// node_modules/@anthropic-ai/sdk/resources/shared.mjs
var init_shared = __esm({
  "node_modules/@anthropic-ai/sdk/resources/shared.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/core/resource.mjs
var APIResource;
var init_resource = __esm({
  "node_modules/@anthropic-ai/sdk/core/resource.mjs"() {
    APIResource = class {
      constructor(client) {
        this._client = client;
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/headers.mjs
function* iterateHeaders(headers) {
  if (!headers)
    return;
  if (brand_privateNullableHeaders in headers) {
    const { values, nulls } = headers;
    yield* values.entries();
    for (const name of nulls) {
      yield [name, null];
    }
    return;
  }
  let shouldClear = false;
  let iter;
  if (headers instanceof Headers) {
    iter = headers.entries();
  } else if (isReadonlyArray(headers)) {
    iter = headers;
  } else {
    shouldClear = true;
    iter = Object.entries(headers ?? {});
  }
  for (let row of iter) {
    const name = row[0];
    if (typeof name !== "string")
      throw new TypeError("expected header name to be a string");
    const values = isReadonlyArray(row[1]) ? row[1] : [row[1]];
    let didClear = false;
    for (const value of values) {
      if (value === void 0)
        continue;
      if (shouldClear && !didClear) {
        didClear = true;
        yield [name, clearSentinel];
      }
      yield [name, value];
    }
  }
}
var brand_privateNullableHeaders, clearSentinel, APPEND_HEADERS, appendHeaderValue, buildHeaders;
var init_headers = __esm({
  "node_modules/@anthropic-ai/sdk/internal/headers.mjs"() {
    init_values();
    brand_privateNullableHeaders = /* @__PURE__ */ Symbol.for("brand.privateNullableHeaders");
    clearSentinel = /* @__PURE__ */ Symbol("clear");
    APPEND_HEADERS = /* @__PURE__ */ new Set(["x-stainless-helper"]);
    appendHeaderValue = (existing, addition) => {
      const tokens = existing ? existing.split(",").map((t) => t.trim()).filter(Boolean) : [];
      for (const tok of addition.split(",").map((t) => t.trim())) {
        if (tok && !tokens.includes(tok))
          tokens.push(tok);
      }
      return tokens.join(", ");
    };
    buildHeaders = (newHeaders) => {
      const targetHeaders = new Headers();
      const nullHeaders = /* @__PURE__ */ new Set();
      for (const headers of newHeaders) {
        const seenHeaders = /* @__PURE__ */ new Set();
        for (const [name, value] of iterateHeaders(headers)) {
          const lowerName = name.toLowerCase();
          if (APPEND_HEADERS.has(lowerName)) {
            if (value === clearSentinel)
              continue;
            if (value === null) {
              targetHeaders.delete(name);
              nullHeaders.add(lowerName);
            } else {
              targetHeaders.set(name, appendHeaderValue(targetHeaders.get(name), value));
              nullHeaders.delete(lowerName);
            }
            continue;
          }
          if (value === clearSentinel || !seenHeaders.has(lowerName)) {
            targetHeaders.delete(name);
            seenHeaders.add(lowerName);
            if (value === clearSentinel)
              continue;
          }
          if (value === null) {
            targetHeaders.delete(name);
            nullHeaders.add(lowerName);
          } else {
            targetHeaders.append(name, value);
            nullHeaders.delete(lowerName);
          }
        }
      }
      return { [brand_privateNullableHeaders]: true, values: targetHeaders, nulls: nullHeaders };
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/path.mjs
function encodeURIPath(str) {
  return str.replace(/[^A-Za-z0-9\-._~!$&'()*+,;=:@]+/g, encodeURIComponent);
}
var EMPTY, createPathTagFunction, path;
var init_path = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/path.mjs"() {
    init_error();
    EMPTY = /* @__PURE__ */ Object.freeze(/* @__PURE__ */ Object.create(null));
    createPathTagFunction = (pathEncoder = encodeURIPath) => function path6(statics, ...params) {
      if (statics.length === 1)
        return statics[0];
      let postPath = false;
      const invalidSegments = [];
      const path7 = statics.reduce((previousValue, currentValue, index) => {
        if (/[?#]/.test(currentValue)) {
          postPath = true;
        }
        const value = params[index];
        let encoded = (postPath ? encodeURIComponent : pathEncoder)("" + value);
        if (index !== params.length && (value == null || typeof value === "object" && // handle values from other realms
        value.toString === Object.getPrototypeOf(Object.getPrototypeOf(value.hasOwnProperty ?? EMPTY) ?? EMPTY)?.toString)) {
          encoded = value + "";
          invalidSegments.push({
            start: previousValue.length + currentValue.length,
            length: encoded.length,
            error: `Value of type ${Object.prototype.toString.call(value).slice(8, -1)} is not a valid path parameter`
          });
        }
        return previousValue + currentValue + (index === params.length ? "" : encoded);
      }, "");
      const pathOnly = path7.split(/[?#]/, 1)[0];
      const invalidSegmentPattern = /(?<=^|\/)(?:\.|%2e){1,2}(?=\/|$)/gi;
      let match;
      while ((match = invalidSegmentPattern.exec(pathOnly)) !== null) {
        invalidSegments.push({
          start: match.index,
          length: match[0].length,
          error: `Value "${match[0]}" can't be safely passed as a path parameter`
        });
      }
      invalidSegments.sort((a, b) => a.start - b.start);
      if (invalidSegments.length > 0) {
        let lastEnd = 0;
        const underline = invalidSegments.reduce((acc, segment) => {
          const spaces = " ".repeat(segment.start - lastEnd);
          const arrows = "^".repeat(segment.length);
          lastEnd = segment.start + segment.length;
          return acc + spaces + arrows;
        }, "");
        throw new AnthropicError(`Path parameters result in path with invalid segments:
${invalidSegments.map((e) => e.error).join("\n")}
${path7}
${underline}`);
      }
      return path7;
    };
    path = /* @__PURE__ */ createPathTagFunction(encodeURIPath);
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/deployment-runs.mjs
var DeploymentRuns;
var init_deployment_runs = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/deployment-runs.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    DeploymentRuns = class extends APIResource {
      /**
       * Get Deployment Run
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeploymentRun =
       *   await client.beta.deploymentRuns.retrieve(
       *     'deployment_run_id',
       *   );
       * ```
       */
      retrieve(deploymentRunID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/deployment_runs/${deploymentRunID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Deployment Runs
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsDeploymentRun of client.beta.deploymentRuns.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/deployment_runs?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/deployments.mjs
var Deployments;
var init_deployments = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/deployments.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Deployments = class extends APIResource {
      /**
       * Create Deployment
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeployment =
       *   await client.beta.deployments.create({
       *     agent: 'string',
       *     environment_id: 'x',
       *     initial_events: [
       *       {
       *         content: [
       *           {
       *             text: 'Where is my order #1234?',
       *             type: 'text',
       *           },
       *         ],
       *         type: 'user.message',
       *       },
       *     ],
       *     name: 'x',
       *   });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/deployments?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get Deployment
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeployment =
       *   await client.beta.deployments.retrieve(
       *     'depl_011CZkZcDH3vPqd7xnEfwTai',
       *   );
       * ```
       */
      retrieve(deploymentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/deployments/${deploymentID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update Deployment
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeployment =
       *   await client.beta.deployments.update(
       *     'depl_011CZkZcDH3vPqd7xnEfwTai',
       *   );
       * ```
       */
      update(deploymentID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/deployments/${deploymentID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Deployments
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsDeployment of client.beta.deployments.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/deployments?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive Deployment
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeployment =
       *   await client.beta.deployments.archive(
       *     'depl_011CZkZcDH3vPqd7xnEfwTai',
       *   );
       * ```
       */
      archive(deploymentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/deployments/${deploymentID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Pause Deployment
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeployment =
       *   await client.beta.deployments.pause(
       *     'depl_011CZkZcDH3vPqd7xnEfwTai',
       *   );
       * ```
       */
      pause(deploymentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/deployments/${deploymentID}/pause?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Run Deployment Now
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeploymentRun =
       *   await client.beta.deployments.run(
       *     'depl_011CZkZcDH3vPqd7xnEfwTai',
       *   );
       * ```
       */
      run(deploymentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/deployments/${deploymentID}/run?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Unpause Deployment
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeployment =
       *   await client.beta.deployments.unpause(
       *     'depl_011CZkZcDH3vPqd7xnEfwTai',
       *   );
       * ```
       */
      unpause(deploymentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/deployments/${deploymentID}/unpause?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/dreams.mjs
var Dreams;
var init_dreams = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/dreams.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Dreams = class extends APIResource {
      /**
       * Create a Dream
       *
       * @example
       * ```ts
       * const betaDream = await client.beta.dreams.create({
       *   inputs: [{ memory_store_id: 'x', type: 'memory_store' }],
       *   model: 'string',
       * });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/dreams?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "dreaming-2026-04-21"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get a Dream
       *
       * @example
       * ```ts
       * const betaDream = await client.beta.dreams.retrieve(
       *   'dream_id',
       * );
       * ```
       */
      retrieve(dreamID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/dreams/${dreamID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "dreaming-2026-04-21"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Dreams
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaDream of client.beta.dreams.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/dreams?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "dreaming-2026-04-21"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive a Dream
       *
       * @example
       * ```ts
       * const betaDream = await client.beta.dreams.archive(
       *   'dream_id',
       * );
       * ```
       */
      archive(dreamID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/dreams/${dreamID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "dreaming-2026-04-21"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Cancel a Dream
       *
       * @example
       * ```ts
       * const betaDream = await client.beta.dreams.cancel(
       *   'dream_id',
       * );
       * ```
       */
      cancel(dreamID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/dreams/${dreamID}/cancel?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "dreaming-2026-04-21"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/stainless-helper-header.mjs
function helperHeader(value) {
  return { [STAINLESS_HELPER_HEADER]: value };
}
function wasCreatedByStainlessHelper(value) {
  return typeof value === "object" && value !== null && SDK_HELPER_SYMBOL in value;
}
function collectStainlessHelpers(tools, messages) {
  const helpers = /* @__PURE__ */ new Set();
  if (tools) {
    for (const tool of tools) {
      if (wasCreatedByStainlessHelper(tool)) {
        helpers.add(tool[SDK_HELPER_SYMBOL]);
      }
    }
  }
  if (messages) {
    for (const message of messages) {
      if (wasCreatedByStainlessHelper(message)) {
        helpers.add(message[SDK_HELPER_SYMBOL]);
      }
      const content = message.content;
      if (Array.isArray(content)) {
        for (const block of content) {
          if (wasCreatedByStainlessHelper(block)) {
            helpers.add(block[SDK_HELPER_SYMBOL]);
          }
        }
      }
    }
  }
  return Array.from(helpers);
}
function stainlessHelperHeader(tools, messages) {
  const helpers = collectStainlessHelpers(tools, messages);
  if (helpers.length === 0)
    return {};
  return { [STAINLESS_HELPER_HEADER]: helpers.join(", ") };
}
function stainlessHelperHeaderFromFile(file) {
  if (wasCreatedByStainlessHelper(file)) {
    return { [STAINLESS_HELPER_HEADER]: file[SDK_HELPER_SYMBOL] };
  }
  return {};
}
var STAINLESS_HELPER_HEADER, STAINLESS_HELPER_METHOD_HEADER, SDK_HELPER_SYMBOL;
var init_stainless_helper_header = __esm({
  "node_modules/@anthropic-ai/sdk/internal/stainless-helper-header.mjs"() {
    STAINLESS_HELPER_HEADER = "x-stainless-helper";
    STAINLESS_HELPER_METHOD_HEADER = "x-stainless-helper-method";
    SDK_HELPER_SYMBOL = /* @__PURE__ */ Symbol("anthropic.sdk.stainlessHelper");
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/files.mjs
var Files;
var init_files = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/files.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_stainless_helper_header();
    init_uploads();
    init_path();
    Files = class extends APIResource {
      /**
       * List Files
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const fileMetadata of client.beta.files.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/files?beta=true", Page, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "files-api-2025-04-14"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete File
       *
       * @example
       * ```ts
       * const deletedFile = await client.beta.files.delete(
       *   'file_id',
       * );
       * ```
       */
      delete(fileID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/files/${fileID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "files-api-2025-04-14"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Download File
       *
       * @example
       * ```ts
       * const response = await client.beta.files.download(
       *   'file_id',
       * );
       *
       * const content = await response.blob();
       * console.log(content);
       * ```
       */
      download(fileID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/files/${fileID}/content?beta=true`, {
          ...options,
          headers: buildHeaders([
            {
              "anthropic-beta": [...betas ?? [], "files-api-2025-04-14"].toString(),
              Accept: "application/binary"
            },
            options?.headers
          ]),
          __binaryResponse: true
        });
      }
      /**
       * Get File Metadata
       *
       * @example
       * ```ts
       * const fileMetadata =
       *   await client.beta.files.retrieveMetadata('file_id');
       * ```
       */
      retrieveMetadata(fileID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/files/${fileID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "files-api-2025-04-14"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Upload File
       *
       * @example
       * ```ts
       * const fileMetadata = await client.beta.files.upload({
       *   file: fs.createReadStream('path/to/file'),
       * });
       * ```
       */
      upload(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/files?beta=true", multipartFormRequestOptions({
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "files-api-2025-04-14"].toString() },
            stainlessHelperHeaderFromFile(body.file),
            options?.headers
          ])
        }, this._client));
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/models.mjs
var Models;
var init_models = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/models.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Models = class extends APIResource {
      /**
       * Get a specific model.
       *
       * The Models API response can be used to determine information about a specific
       * model or resolve a model alias to a model ID.
       *
       * @example
       * ```ts
       * const betaModelInfo = await client.beta.models.retrieve(
       *   'model_id',
       * );
       * ```
       */
      retrieve(modelID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/models/${modelID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { ...betas?.toString() != null ? { "anthropic-beta": betas?.toString() } : void 0 },
            options?.headers
          ])
        });
      }
      /**
       * List available models.
       *
       * The Models API response can be used to determine which models are available for
       * use in the API. More recently released models are listed first.
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaModelInfo of client.beta.models.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/models?beta=true", Page, {
          query,
          ...options,
          headers: buildHeaders([
            { ...betas?.toString() != null ? { "anthropic-beta": betas?.toString() } : void 0 },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/user-profiles.mjs
var UserProfiles;
var init_user_profiles = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/user-profiles.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    UserProfiles = class extends APIResource {
      /**
       * Create User Profile
       *
       * @example
       * ```ts
       * const betaUserProfile =
       *   await client.beta.userProfiles.create();
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/user_profiles?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "user-profiles-2026-03-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get User Profile
       *
       * @example
       * ```ts
       * const betaUserProfile =
       *   await client.beta.userProfiles.retrieve(
       *     'uprof_011CZkZCu8hGbp5mYRQgUmz9',
       *   );
       * ```
       */
      retrieve(userProfileID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/user_profiles/${userProfileID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "user-profiles-2026-03-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update User Profile
       *
       * @example
       * ```ts
       * const betaUserProfile =
       *   await client.beta.userProfiles.update(
       *     'uprof_011CZkZCu8hGbp5mYRQgUmz9',
       *   );
       * ```
       */
      update(userProfileID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/user_profiles/${userProfileID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "user-profiles-2026-03-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List User Profiles
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaUserProfile of client.beta.userProfiles.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/user_profiles?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "user-profiles-2026-03-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Create Enrollment URL
       *
       * @example
       * ```ts
       * const betaUserProfileEnrollmentURL =
       *   await client.beta.userProfiles.createEnrollmentURL(
       *     'uprof_011CZkZCu8hGbp5mYRQgUmz9',
       *   );
       * ```
       */
      createEnrollmentURL(userProfileID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/user_profiles/${userProfileID}/enrollment_url?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "user-profiles-2026-03-24"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/standardwebhooks/dist/timing_safe_equal.js
var require_timing_safe_equal = __commonJS({
  "node_modules/standardwebhooks/dist/timing_safe_equal.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    exports.timingSafeEqual = void 0;
    function assert(expr, msg = "") {
      if (!expr) {
        throw new Error(msg);
      }
    }
    function timingSafeEqual(a, b) {
      if (a.byteLength !== b.byteLength) {
        return false;
      }
      if (!(a instanceof DataView)) {
        a = new DataView(ArrayBuffer.isView(a) ? a.buffer : a);
      }
      if (!(b instanceof DataView)) {
        b = new DataView(ArrayBuffer.isView(b) ? b.buffer : b);
      }
      assert(a instanceof DataView);
      assert(b instanceof DataView);
      const length = a.byteLength;
      let out = 0;
      let i = -1;
      while (++i < length) {
        out |= a.getUint8(i) ^ b.getUint8(i);
      }
      return out === 0;
    }
    exports.timingSafeEqual = timingSafeEqual;
  }
});

// node_modules/@stablelib/base64/lib/base64.js
var require_base64 = __commonJS({
  "node_modules/@stablelib/base64/lib/base64.js"(exports) {
    "use strict";
    var __extends = exports && exports.__extends || /* @__PURE__ */ (function() {
      var extendStatics = function(d, b) {
        extendStatics = Object.setPrototypeOf || { __proto__: [] } instanceof Array && function(d2, b2) {
          d2.__proto__ = b2;
        } || function(d2, b2) {
          for (var p in b2) if (b2.hasOwnProperty(p)) d2[p] = b2[p];
        };
        return extendStatics(d, b);
      };
      return function(d, b) {
        extendStatics(d, b);
        function __() {
          this.constructor = d;
        }
        d.prototype = b === null ? Object.create(b) : (__.prototype = b.prototype, new __());
      };
    })();
    Object.defineProperty(exports, "__esModule", { value: true });
    var INVALID_BYTE = 256;
    var Coder = (
      /** @class */
      (function() {
        function Coder2(_paddingCharacter) {
          if (_paddingCharacter === void 0) {
            _paddingCharacter = "=";
          }
          this._paddingCharacter = _paddingCharacter;
        }
        Coder2.prototype.encodedLength = function(length) {
          if (!this._paddingCharacter) {
            return (length * 8 + 5) / 6 | 0;
          }
          return (length + 2) / 3 * 4 | 0;
        };
        Coder2.prototype.encode = function(data) {
          var out = "";
          var i = 0;
          for (; i < data.length - 2; i += 3) {
            var c = data[i] << 16 | data[i + 1] << 8 | data[i + 2];
            out += this._encodeByte(c >>> 3 * 6 & 63);
            out += this._encodeByte(c >>> 2 * 6 & 63);
            out += this._encodeByte(c >>> 1 * 6 & 63);
            out += this._encodeByte(c >>> 0 * 6 & 63);
          }
          var left = data.length - i;
          if (left > 0) {
            var c = data[i] << 16 | (left === 2 ? data[i + 1] << 8 : 0);
            out += this._encodeByte(c >>> 3 * 6 & 63);
            out += this._encodeByte(c >>> 2 * 6 & 63);
            if (left === 2) {
              out += this._encodeByte(c >>> 1 * 6 & 63);
            } else {
              out += this._paddingCharacter || "";
            }
            out += this._paddingCharacter || "";
          }
          return out;
        };
        Coder2.prototype.maxDecodedLength = function(length) {
          if (!this._paddingCharacter) {
            return (length * 6 + 7) / 8 | 0;
          }
          return length / 4 * 3 | 0;
        };
        Coder2.prototype.decodedLength = function(s) {
          return this.maxDecodedLength(s.length - this._getPaddingLength(s));
        };
        Coder2.prototype.decode = function(s) {
          if (s.length === 0) {
            return new Uint8Array(0);
          }
          var paddingLength = this._getPaddingLength(s);
          var length = s.length - paddingLength;
          var out = new Uint8Array(this.maxDecodedLength(length));
          var op = 0;
          var i = 0;
          var haveBad = 0;
          var v0 = 0, v1 = 0, v2 = 0, v3 = 0;
          for (; i < length - 4; i += 4) {
            v0 = this._decodeChar(s.charCodeAt(i + 0));
            v1 = this._decodeChar(s.charCodeAt(i + 1));
            v2 = this._decodeChar(s.charCodeAt(i + 2));
            v3 = this._decodeChar(s.charCodeAt(i + 3));
            out[op++] = v0 << 2 | v1 >>> 4;
            out[op++] = v1 << 4 | v2 >>> 2;
            out[op++] = v2 << 6 | v3;
            haveBad |= v0 & INVALID_BYTE;
            haveBad |= v1 & INVALID_BYTE;
            haveBad |= v2 & INVALID_BYTE;
            haveBad |= v3 & INVALID_BYTE;
          }
          if (i < length - 1) {
            v0 = this._decodeChar(s.charCodeAt(i));
            v1 = this._decodeChar(s.charCodeAt(i + 1));
            out[op++] = v0 << 2 | v1 >>> 4;
            haveBad |= v0 & INVALID_BYTE;
            haveBad |= v1 & INVALID_BYTE;
          }
          if (i < length - 2) {
            v2 = this._decodeChar(s.charCodeAt(i + 2));
            out[op++] = v1 << 4 | v2 >>> 2;
            haveBad |= v2 & INVALID_BYTE;
          }
          if (i < length - 3) {
            v3 = this._decodeChar(s.charCodeAt(i + 3));
            out[op++] = v2 << 6 | v3;
            haveBad |= v3 & INVALID_BYTE;
          }
          if (haveBad !== 0) {
            throw new Error("Base64Coder: incorrect characters for decoding");
          }
          return out;
        };
        Coder2.prototype._encodeByte = function(b) {
          var result = b;
          result += 65;
          result += 25 - b >>> 8 & 0 - 65 - 26 + 97;
          result += 51 - b >>> 8 & 26 - 97 - 52 + 48;
          result += 61 - b >>> 8 & 52 - 48 - 62 + 43;
          result += 62 - b >>> 8 & 62 - 43 - 63 + 47;
          return String.fromCharCode(result);
        };
        Coder2.prototype._decodeChar = function(c) {
          var result = INVALID_BYTE;
          result += (42 - c & c - 44) >>> 8 & -INVALID_BYTE + c - 43 + 62;
          result += (46 - c & c - 48) >>> 8 & -INVALID_BYTE + c - 47 + 63;
          result += (47 - c & c - 58) >>> 8 & -INVALID_BYTE + c - 48 + 52;
          result += (64 - c & c - 91) >>> 8 & -INVALID_BYTE + c - 65 + 0;
          result += (96 - c & c - 123) >>> 8 & -INVALID_BYTE + c - 97 + 26;
          return result;
        };
        Coder2.prototype._getPaddingLength = function(s) {
          var paddingLength = 0;
          if (this._paddingCharacter) {
            for (var i = s.length - 1; i >= 0; i--) {
              if (s[i] !== this._paddingCharacter) {
                break;
              }
              paddingLength++;
            }
            if (s.length < 4 || paddingLength > 2) {
              throw new Error("Base64Coder: incorrect padding");
            }
          }
          return paddingLength;
        };
        return Coder2;
      })()
    );
    exports.Coder = Coder;
    var stdCoder = new Coder();
    function encode2(data) {
      return stdCoder.encode(data);
    }
    exports.encode = encode2;
    function decode(s) {
      return stdCoder.decode(s);
    }
    exports.decode = decode;
    var URLSafeCoder = (
      /** @class */
      (function(_super) {
        __extends(URLSafeCoder2, _super);
        function URLSafeCoder2() {
          return _super !== null && _super.apply(this, arguments) || this;
        }
        URLSafeCoder2.prototype._encodeByte = function(b) {
          var result = b;
          result += 65;
          result += 25 - b >>> 8 & 0 - 65 - 26 + 97;
          result += 51 - b >>> 8 & 26 - 97 - 52 + 48;
          result += 61 - b >>> 8 & 52 - 48 - 62 + 45;
          result += 62 - b >>> 8 & 62 - 45 - 63 + 95;
          return String.fromCharCode(result);
        };
        URLSafeCoder2.prototype._decodeChar = function(c) {
          var result = INVALID_BYTE;
          result += (44 - c & c - 46) >>> 8 & -INVALID_BYTE + c - 45 + 62;
          result += (94 - c & c - 96) >>> 8 & -INVALID_BYTE + c - 95 + 63;
          result += (47 - c & c - 58) >>> 8 & -INVALID_BYTE + c - 48 + 52;
          result += (64 - c & c - 91) >>> 8 & -INVALID_BYTE + c - 65 + 0;
          result += (96 - c & c - 123) >>> 8 & -INVALID_BYTE + c - 97 + 26;
          return result;
        };
        return URLSafeCoder2;
      })(Coder)
    );
    exports.URLSafeCoder = URLSafeCoder;
    var urlSafeCoder = new URLSafeCoder();
    function encodeURLSafe(data) {
      return urlSafeCoder.encode(data);
    }
    exports.encodeURLSafe = encodeURLSafe;
    function decodeURLSafe(s) {
      return urlSafeCoder.decode(s);
    }
    exports.decodeURLSafe = decodeURLSafe;
    exports.encodedLength = function(length) {
      return stdCoder.encodedLength(length);
    };
    exports.maxDecodedLength = function(length) {
      return stdCoder.maxDecodedLength(length);
    };
    exports.decodedLength = function(s) {
      return stdCoder.decodedLength(s);
    };
  }
});

// node_modules/fast-sha256/sha256.js
var require_sha256 = __commonJS({
  "node_modules/fast-sha256/sha256.js"(exports, module) {
    (function(root, factory) {
      var exports2 = {};
      factory(exports2);
      var sha256 = exports2["default"];
      for (var k in exports2) {
        sha256[k] = exports2[k];
      }
      if (typeof module === "object" && typeof module.exports === "object") {
        module.exports = sha256;
      } else if (typeof define === "function" && define.amd) {
        define(function() {
          return sha256;
        });
      } else {
        root.sha256 = sha256;
      }
    })(exports, function(exports2) {
      "use strict";
      exports2.__esModule = true;
      exports2.digestLength = 32;
      exports2.blockSize = 64;
      var K = new Uint32Array([
        1116352408,
        1899447441,
        3049323471,
        3921009573,
        961987163,
        1508970993,
        2453635748,
        2870763221,
        3624381080,
        310598401,
        607225278,
        1426881987,
        1925078388,
        2162078206,
        2614888103,
        3248222580,
        3835390401,
        4022224774,
        264347078,
        604807628,
        770255983,
        1249150122,
        1555081692,
        1996064986,
        2554220882,
        2821834349,
        2952996808,
        3210313671,
        3336571891,
        3584528711,
        113926993,
        338241895,
        666307205,
        773529912,
        1294757372,
        1396182291,
        1695183700,
        1986661051,
        2177026350,
        2456956037,
        2730485921,
        2820302411,
        3259730800,
        3345764771,
        3516065817,
        3600352804,
        4094571909,
        275423344,
        430227734,
        506948616,
        659060556,
        883997877,
        958139571,
        1322822218,
        1537002063,
        1747873779,
        1955562222,
        2024104815,
        2227730452,
        2361852424,
        2428436474,
        2756734187,
        3204031479,
        3329325298
      ]);
      function hashBlocks(w, v, p, pos, len) {
        var a, b, c, d, e, f, g, h, u, i, j, t1, t2;
        while (len >= 64) {
          a = v[0];
          b = v[1];
          c = v[2];
          d = v[3];
          e = v[4];
          f = v[5];
          g = v[6];
          h = v[7];
          for (i = 0; i < 16; i++) {
            j = pos + i * 4;
            w[i] = (p[j] & 255) << 24 | (p[j + 1] & 255) << 16 | (p[j + 2] & 255) << 8 | p[j + 3] & 255;
          }
          for (i = 16; i < 64; i++) {
            u = w[i - 2];
            t1 = (u >>> 17 | u << 32 - 17) ^ (u >>> 19 | u << 32 - 19) ^ u >>> 10;
            u = w[i - 15];
            t2 = (u >>> 7 | u << 32 - 7) ^ (u >>> 18 | u << 32 - 18) ^ u >>> 3;
            w[i] = (t1 + w[i - 7] | 0) + (t2 + w[i - 16] | 0);
          }
          for (i = 0; i < 64; i++) {
            t1 = (((e >>> 6 | e << 32 - 6) ^ (e >>> 11 | e << 32 - 11) ^ (e >>> 25 | e << 32 - 25)) + (e & f ^ ~e & g) | 0) + (h + (K[i] + w[i] | 0) | 0) | 0;
            t2 = ((a >>> 2 | a << 32 - 2) ^ (a >>> 13 | a << 32 - 13) ^ (a >>> 22 | a << 32 - 22)) + (a & b ^ a & c ^ b & c) | 0;
            h = g;
            g = f;
            f = e;
            e = d + t1 | 0;
            d = c;
            c = b;
            b = a;
            a = t1 + t2 | 0;
          }
          v[0] += a;
          v[1] += b;
          v[2] += c;
          v[3] += d;
          v[4] += e;
          v[5] += f;
          v[6] += g;
          v[7] += h;
          pos += 64;
          len -= 64;
        }
        return pos;
      }
      var Hash = (
        /** @class */
        (function() {
          function Hash2() {
            this.digestLength = exports2.digestLength;
            this.blockSize = exports2.blockSize;
            this.state = new Int32Array(8);
            this.temp = new Int32Array(64);
            this.buffer = new Uint8Array(128);
            this.bufferLength = 0;
            this.bytesHashed = 0;
            this.finished = false;
            this.reset();
          }
          Hash2.prototype.reset = function() {
            this.state[0] = 1779033703;
            this.state[1] = 3144134277;
            this.state[2] = 1013904242;
            this.state[3] = 2773480762;
            this.state[4] = 1359893119;
            this.state[5] = 2600822924;
            this.state[6] = 528734635;
            this.state[7] = 1541459225;
            this.bufferLength = 0;
            this.bytesHashed = 0;
            this.finished = false;
            return this;
          };
          Hash2.prototype.clean = function() {
            for (var i = 0; i < this.buffer.length; i++) {
              this.buffer[i] = 0;
            }
            for (var i = 0; i < this.temp.length; i++) {
              this.temp[i] = 0;
            }
            this.reset();
          };
          Hash2.prototype.update = function(data, dataLength) {
            if (dataLength === void 0) {
              dataLength = data.length;
            }
            if (this.finished) {
              throw new Error("SHA256: can't update because hash was finished.");
            }
            var dataPos = 0;
            this.bytesHashed += dataLength;
            if (this.bufferLength > 0) {
              while (this.bufferLength < 64 && dataLength > 0) {
                this.buffer[this.bufferLength++] = data[dataPos++];
                dataLength--;
              }
              if (this.bufferLength === 64) {
                hashBlocks(this.temp, this.state, this.buffer, 0, 64);
                this.bufferLength = 0;
              }
            }
            if (dataLength >= 64) {
              dataPos = hashBlocks(this.temp, this.state, data, dataPos, dataLength);
              dataLength %= 64;
            }
            while (dataLength > 0) {
              this.buffer[this.bufferLength++] = data[dataPos++];
              dataLength--;
            }
            return this;
          };
          Hash2.prototype.finish = function(out) {
            if (!this.finished) {
              var bytesHashed = this.bytesHashed;
              var left = this.bufferLength;
              var bitLenHi = bytesHashed / 536870912 | 0;
              var bitLenLo = bytesHashed << 3;
              var padLength = bytesHashed % 64 < 56 ? 64 : 128;
              this.buffer[left] = 128;
              for (var i = left + 1; i < padLength - 8; i++) {
                this.buffer[i] = 0;
              }
              this.buffer[padLength - 8] = bitLenHi >>> 24 & 255;
              this.buffer[padLength - 7] = bitLenHi >>> 16 & 255;
              this.buffer[padLength - 6] = bitLenHi >>> 8 & 255;
              this.buffer[padLength - 5] = bitLenHi >>> 0 & 255;
              this.buffer[padLength - 4] = bitLenLo >>> 24 & 255;
              this.buffer[padLength - 3] = bitLenLo >>> 16 & 255;
              this.buffer[padLength - 2] = bitLenLo >>> 8 & 255;
              this.buffer[padLength - 1] = bitLenLo >>> 0 & 255;
              hashBlocks(this.temp, this.state, this.buffer, 0, padLength);
              this.finished = true;
            }
            for (var i = 0; i < 8; i++) {
              out[i * 4 + 0] = this.state[i] >>> 24 & 255;
              out[i * 4 + 1] = this.state[i] >>> 16 & 255;
              out[i * 4 + 2] = this.state[i] >>> 8 & 255;
              out[i * 4 + 3] = this.state[i] >>> 0 & 255;
            }
            return this;
          };
          Hash2.prototype.digest = function() {
            var out = new Uint8Array(this.digestLength);
            this.finish(out);
            return out;
          };
          Hash2.prototype._saveState = function(out) {
            for (var i = 0; i < this.state.length; i++) {
              out[i] = this.state[i];
            }
          };
          Hash2.prototype._restoreState = function(from, bytesHashed) {
            for (var i = 0; i < this.state.length; i++) {
              this.state[i] = from[i];
            }
            this.bytesHashed = bytesHashed;
            this.finished = false;
            this.bufferLength = 0;
          };
          return Hash2;
        })()
      );
      exports2.Hash = Hash;
      var HMAC = (
        /** @class */
        (function() {
          function HMAC2(key) {
            this.inner = new Hash();
            this.outer = new Hash();
            this.blockSize = this.inner.blockSize;
            this.digestLength = this.inner.digestLength;
            var pad = new Uint8Array(this.blockSize);
            if (key.length > this.blockSize) {
              new Hash().update(key).finish(pad).clean();
            } else {
              for (var i = 0; i < key.length; i++) {
                pad[i] = key[i];
              }
            }
            for (var i = 0; i < pad.length; i++) {
              pad[i] ^= 54;
            }
            this.inner.update(pad);
            for (var i = 0; i < pad.length; i++) {
              pad[i] ^= 54 ^ 92;
            }
            this.outer.update(pad);
            this.istate = new Uint32Array(8);
            this.ostate = new Uint32Array(8);
            this.inner._saveState(this.istate);
            this.outer._saveState(this.ostate);
            for (var i = 0; i < pad.length; i++) {
              pad[i] = 0;
            }
          }
          HMAC2.prototype.reset = function() {
            this.inner._restoreState(this.istate, this.inner.blockSize);
            this.outer._restoreState(this.ostate, this.outer.blockSize);
            return this;
          };
          HMAC2.prototype.clean = function() {
            for (var i = 0; i < this.istate.length; i++) {
              this.ostate[i] = this.istate[i] = 0;
            }
            this.inner.clean();
            this.outer.clean();
          };
          HMAC2.prototype.update = function(data) {
            this.inner.update(data);
            return this;
          };
          HMAC2.prototype.finish = function(out) {
            if (this.outer.finished) {
              this.outer.finish(out);
            } else {
              this.inner.finish(out);
              this.outer.update(out, this.digestLength).finish(out);
            }
            return this;
          };
          HMAC2.prototype.digest = function() {
            var out = new Uint8Array(this.digestLength);
            this.finish(out);
            return out;
          };
          return HMAC2;
        })()
      );
      exports2.HMAC = HMAC;
      function hash(data) {
        var h = new Hash().update(data);
        var digest = h.digest();
        h.clean();
        return digest;
      }
      exports2.hash = hash;
      exports2["default"] = hash;
      function hmac(key, data) {
        var h = new HMAC(key).update(data);
        var digest = h.digest();
        h.clean();
        return digest;
      }
      exports2.hmac = hmac;
      function fillBuffer(buffer, hmac2, info, counter) {
        var num = counter[0];
        if (num === 0) {
          throw new Error("hkdf: cannot expand more");
        }
        hmac2.reset();
        if (num > 1) {
          hmac2.update(buffer);
        }
        if (info) {
          hmac2.update(info);
        }
        hmac2.update(counter);
        hmac2.finish(buffer);
        counter[0]++;
      }
      var hkdfSalt = new Uint8Array(exports2.digestLength);
      function hkdf(key, salt, info, length) {
        if (salt === void 0) {
          salt = hkdfSalt;
        }
        if (length === void 0) {
          length = 32;
        }
        var counter = new Uint8Array([1]);
        var okm = hmac(salt, key);
        var hmac_ = new HMAC(okm);
        var buffer = new Uint8Array(hmac_.digestLength);
        var bufpos = buffer.length;
        var out = new Uint8Array(length);
        for (var i = 0; i < length; i++) {
          if (bufpos === buffer.length) {
            fillBuffer(buffer, hmac_, info, counter);
            bufpos = 0;
          }
          out[i] = buffer[bufpos++];
        }
        hmac_.clean();
        buffer.fill(0);
        counter.fill(0);
        return out;
      }
      exports2.hkdf = hkdf;
      function pbkdf2(password, salt, iterations, dkLen) {
        var prf = new HMAC(password);
        var len = prf.digestLength;
        var ctr = new Uint8Array(4);
        var t = new Uint8Array(len);
        var u = new Uint8Array(len);
        var dk = new Uint8Array(dkLen);
        for (var i = 0; i * len < dkLen; i++) {
          var c = i + 1;
          ctr[0] = c >>> 24 & 255;
          ctr[1] = c >>> 16 & 255;
          ctr[2] = c >>> 8 & 255;
          ctr[3] = c >>> 0 & 255;
          prf.reset();
          prf.update(salt);
          prf.update(ctr);
          prf.finish(u);
          for (var j = 0; j < len; j++) {
            t[j] = u[j];
          }
          for (var j = 2; j <= iterations; j++) {
            prf.reset();
            prf.update(u).finish(u);
            for (var k = 0; k < len; k++) {
              t[k] ^= u[k];
            }
          }
          for (var j = 0; j < len && i * len + j < dkLen; j++) {
            dk[i * len + j] = t[j];
          }
        }
        for (var i = 0; i < len; i++) {
          t[i] = u[i] = 0;
        }
        for (var i = 0; i < 4; i++) {
          ctr[i] = 0;
        }
        prf.clean();
        return dk;
      }
      exports2.pbkdf2 = pbkdf2;
    });
  }
});

// node_modules/standardwebhooks/dist/index.js
var require_dist = __commonJS({
  "node_modules/standardwebhooks/dist/index.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    exports.Webhook = exports.WebhookVerificationError = void 0;
    var timing_safe_equal_1 = require_timing_safe_equal();
    var base64 = require_base64();
    var sha256 = require_sha256();
    var WEBHOOK_TOLERANCE_IN_SECONDS = 5 * 60;
    var ExtendableError = class _ExtendableError extends Error {
      constructor(message) {
        super(message);
        Object.setPrototypeOf(this, _ExtendableError.prototype);
        this.name = "ExtendableError";
        this.stack = new Error(message).stack;
      }
    };
    var WebhookVerificationError = class _WebhookVerificationError extends ExtendableError {
      constructor(message) {
        super(message);
        Object.setPrototypeOf(this, _WebhookVerificationError.prototype);
        this.name = "WebhookVerificationError";
      }
    };
    exports.WebhookVerificationError = WebhookVerificationError;
    var Webhook2 = class _Webhook {
      constructor(secret, options) {
        if (!secret) {
          throw new Error("Secret can't be empty.");
        }
        if ((options === null || options === void 0 ? void 0 : options.format) === "raw") {
          if (secret instanceof Uint8Array) {
            this.key = secret;
          } else {
            this.key = Uint8Array.from(secret, (c) => c.charCodeAt(0));
          }
        } else {
          if (typeof secret !== "string") {
            throw new Error("Expected secret to be of type string");
          }
          if (secret.startsWith(_Webhook.prefix)) {
            secret = secret.substring(_Webhook.prefix.length);
          }
          this.key = base64.decode(secret);
        }
      }
      verify(payload, headers_) {
        const headers = {};
        for (const key of Object.keys(headers_)) {
          headers[key.toLowerCase()] = headers_[key];
        }
        const msgId = headers["webhook-id"];
        const msgSignature = headers["webhook-signature"];
        const msgTimestamp = headers["webhook-timestamp"];
        if (!msgSignature || !msgId || !msgTimestamp) {
          throw new WebhookVerificationError("Missing required headers");
        }
        const timestamp = this.verifyTimestamp(msgTimestamp);
        const computedSignature = this.sign(msgId, timestamp, payload);
        const expectedSignature = computedSignature.split(",")[1];
        const passedSignatures = msgSignature.split(" ");
        const encoder2 = new globalThis.TextEncoder();
        for (const versionedSignature of passedSignatures) {
          const [version, signature] = versionedSignature.split(",");
          if (version !== "v1") {
            continue;
          }
          if ((0, timing_safe_equal_1.timingSafeEqual)(encoder2.encode(signature), encoder2.encode(expectedSignature))) {
            return JSON.parse(payload.toString());
          }
        }
        throw new WebhookVerificationError("No matching signature found");
      }
      sign(msgId, timestamp, payload) {
        if (typeof payload === "string") {
        } else if (payload.constructor.name === "Buffer") {
          payload = payload.toString();
        } else {
          throw new Error("Expected payload to be of type string or Buffer.");
        }
        const encoder2 = new TextEncoder();
        const timestampNumber = Math.floor(timestamp.getTime() / 1e3);
        const toSign = encoder2.encode(`${msgId}.${timestampNumber}.${payload}`);
        const expectedSignature = base64.encode(sha256.hmac(this.key, toSign));
        return `v1,${expectedSignature}`;
      }
      verifyTimestamp(timestampHeader) {
        const now = Math.floor(Date.now() / 1e3);
        const timestamp = parseInt(timestampHeader, 10);
        if (isNaN(timestamp)) {
          throw new WebhookVerificationError("Invalid Signature Headers");
        }
        if (now - timestamp > WEBHOOK_TOLERANCE_IN_SECONDS) {
          throw new WebhookVerificationError("Message timestamp too old");
        }
        if (timestamp > now + WEBHOOK_TOLERANCE_IN_SECONDS) {
          throw new WebhookVerificationError("Message timestamp too new");
        }
        return new Date(timestamp * 1e3);
      }
    };
    exports.Webhook = Webhook2;
    Webhook2.prefix = "whsec_";
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/webhooks.mjs
var import_standardwebhooks, Webhooks;
var init_webhooks = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/webhooks.mjs"() {
    init_resource();
    import_standardwebhooks = __toESM(require_dist(), 1);
    Webhooks = class extends APIResource {
      unwrap(body, { headers, key }) {
        if (headers !== void 0) {
          const keyStr = key === void 0 ? this._client.webhookKey : key;
          if (keyStr === null)
            throw new Error("Webhook key must not be null in order to unwrap");
          const wh = new import_standardwebhooks.Webhook(keyStr);
          wh.verify(body, headers);
        }
        return JSON.parse(body);
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/agents/versions.mjs
var Versions;
var init_versions = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/agents/versions.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Versions = class extends APIResource {
      /**
       * List Agent Versions
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsAgent of client.beta.agents.versions.list(
       *   'agent_011CZkYpogX7uDKUyvBTophP',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(agentID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/agents/${agentID}/versions?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/agents/agents.mjs
var Agents;
var init_agents = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/agents/agents.mjs"() {
    init_resource();
    init_versions();
    init_versions();
    init_pagination();
    init_headers();
    init_path();
    Agents = class extends APIResource {
      constructor() {
        super(...arguments);
        this.versions = new Versions(this._client);
      }
      /**
       * Create Agent
       *
       * @example
       * ```ts
       * const betaManagedAgentsAgent =
       *   await client.beta.agents.create({
       *     model: 'claude-sonnet-4-6',
       *     name: 'My First Agent',
       *   });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/agents?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get Agent
       *
       * @example
       * ```ts
       * const betaManagedAgentsAgent =
       *   await client.beta.agents.retrieve(
       *     'agent_011CZkYpogX7uDKUyvBTophP',
       *   );
       * ```
       */
      retrieve(agentID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.get(path`/v1/agents/${agentID}?beta=true`, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update Agent
       *
       * @example
       * ```ts
       * const betaManagedAgentsAgent =
       *   await client.beta.agents.update(
       *     'agent_011CZkYpogX7uDKUyvBTophP',
       *     { description: 'updated' },
       *   );
       * ```
       */
      update(agentID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/agents/${agentID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Agents
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsAgent of client.beta.agents.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/agents?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive Agent
       *
       * @example
       * ```ts
       * const betaManagedAgentsAgent =
       *   await client.beta.agents.archive(
       *     'agent_011CZkYpogX7uDKUyvBTophP',
       *   );
       * ```
       */
      archive(agentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/agents/${agentID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
    Agents.Versions = Versions;
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/abort.mjs
function linkAbort(external, controller) {
  if (!external)
    return () => {
    };
  if (external.aborted) {
    controller.abort();
    return () => {
    };
  }
  const onAbort = () => controller.abort();
  external.addEventListener("abort", onAbort);
  return () => external.removeEventListener("abort", onAbort);
}
var init_abort = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/abort.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/backoff.mjs
function isStatus(e, code) {
  return e instanceof APIError && e.status === code;
}
function is4xx(e) {
  return e instanceof APIError && typeof e.status === "number" && e.status >= 400 && e.status < 500;
}
function isFatal4xx(e) {
  return is4xx(e) && !isStatus(e, 408) && !isStatus(e, 409) && !isStatus(e, 429);
}
function backoff(attempt, baseMs, capMs) {
  return Math.min(baseMs * 2 ** attempt, capMs);
}
function jitter(lowMs, highMs) {
  return lowMs + Math.random() * (highMs - lowMs);
}
function applyJitter(ms) {
  return ms * (1 - Math.random() * 0.25);
}
var init_backoff = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/backoff.mjs"() {
    init_error();
  }
});

// node_modules/@anthropic-ai/sdk/lib/helper-client.mjs
function copyClientForHelper(client, { authToken, helper }) {
  if (!authToken) {
    throw new AnthropicError(`copyClientForHelper: expected a non-empty authToken but received ${JSON.stringify(authToken)}`);
  }
  const internal = client;
  const parentDefaults = internal._options.defaultHeaders;
  const parentAuthExtraHeaders = internal._authState?.extraHeaders;
  const inheritedAuthExtraHeaders = parentAuthExtraHeaders ? Object.fromEntries(Object.entries(parentAuthExtraHeaders).filter(([name]) => {
    const lower = name.toLowerCase();
    return lower !== "authorization" && lower !== "x-api-key";
  })) : void 0;
  const defaultHeaders = buildHeaders([
    inheritedAuthExtraHeaders,
    parentDefaults,
    { [STAINLESS_HELPER_HEADER]: helper }
  ]);
  return client.withOptions({
    apiKey: null,
    authToken,
    baseURL: client.baseURL,
    credentials: void 0,
    defaultHeaders
  });
}
var init_helper_client = __esm({
  "node_modules/@anthropic-ai/sdk/lib/helper-client.mjs"() {
    init_error();
    init_headers();
    init_stainless_helper_header();
  }
});

// node_modules/@anthropic-ai/sdk/lib/environments/poller.mjs
function backoff2(attempt) {
  return backoff(attempt, POLL_BACKOFF_BASE_MS, POLL_BACKOFF_CAP_MS);
}
function defaultWorkerId() {
  const env = globalThis.process?.env;
  const host = env?.["HOSTNAME"];
  return host ? `${host}-${uuid4()}` : uuid4();
}
var _WorkPoller_runnerClient, _WorkPoller_consumed, _WorkPoller_controller, _WorkPoller_detachExternal, _WorkPoller_autoStop, _WorkPoller_drain, _WorkPoller_blockMs, _WorkPoller_reclaimOlderThanMs, _WorkPoller_requestOpts, POLL_BLOCK_MS, POLL_BACKOFF_BASE_MS, POLL_BACKOFF_CAP_MS, WorkPoller;
var init_poller = __esm({
  "node_modules/@anthropic-ai/sdk/lib/environments/poller.mjs"() {
    init_tslib();
    init_error();
    init_log();
    init_sleep();
    init_uuid();
    init_abort();
    init_headers();
    init_backoff();
    init_helper_client();
    init_backoff();
    POLL_BLOCK_MS = 999;
    POLL_BACKOFF_BASE_MS = 1e3;
    POLL_BACKOFF_CAP_MS = 6e4;
    WorkPoller = class {
      constructor(opts) {
        _WorkPoller_runnerClient.set(this, void 0);
        _WorkPoller_consumed.set(this, false);
        _WorkPoller_controller.set(this, void 0);
        _WorkPoller_detachExternal.set(this, void 0);
        _WorkPoller_autoStop.set(this, void 0);
        _WorkPoller_drain.set(this, void 0);
        _WorkPoller_blockMs.set(this, void 0);
        _WorkPoller_reclaimOlderThanMs.set(this, void 0);
        _WorkPoller_requestOpts.set(this, void 0);
        this.client = opts.client;
        this.environmentId = opts.environmentId;
        this.environmentKey = opts.environmentKey;
        this.workerId = opts.workerId ?? defaultWorkerId();
        __classPrivateFieldSet(this, _WorkPoller_runnerClient, copyClientForHelper(opts.client, {
          authToken: opts.environmentKey,
          helper: "environments-work-poller"
        }), "f");
        __classPrivateFieldSet(this, _WorkPoller_autoStop, opts.autoStop ?? true, "f");
        __classPrivateFieldSet(this, _WorkPoller_drain, opts.drain ?? false, "f");
        __classPrivateFieldSet(this, _WorkPoller_blockMs, opts.blockMs === void 0 ? POLL_BLOCK_MS : opts.blockMs, "f");
        __classPrivateFieldSet(this, _WorkPoller_reclaimOlderThanMs, opts.reclaimOlderThanMs ?? null, "f");
        __classPrivateFieldSet(this, _WorkPoller_requestOpts, opts.requestOptions, "f");
        __classPrivateFieldSet(this, _WorkPoller_controller, new AbortController(), "f");
        __classPrivateFieldSet(this, _WorkPoller_detachExternal, linkAbort(opts.signal, __classPrivateFieldGet(this, _WorkPoller_controller, "f")), "f");
      }
      /** Read-only view of this iterator's abort signal. */
      get signal() {
        return __classPrivateFieldGet(this, _WorkPoller_controller, "f").signal;
      }
      /** Abort the iterator. The current `for await` will exit cleanly. */
      abort() {
        __classPrivateFieldGet(this, _WorkPoller_controller, "f").abort();
      }
      async *[(_WorkPoller_runnerClient = /* @__PURE__ */ new WeakMap(), _WorkPoller_consumed = /* @__PURE__ */ new WeakMap(), _WorkPoller_controller = /* @__PURE__ */ new WeakMap(), _WorkPoller_detachExternal = /* @__PURE__ */ new WeakMap(), _WorkPoller_autoStop = /* @__PURE__ */ new WeakMap(), _WorkPoller_drain = /* @__PURE__ */ new WeakMap(), _WorkPoller_blockMs = /* @__PURE__ */ new WeakMap(), _WorkPoller_reclaimOlderThanMs = /* @__PURE__ */ new WeakMap(), _WorkPoller_requestOpts = /* @__PURE__ */ new WeakMap(), Symbol.asyncIterator)]() {
        if (__classPrivateFieldGet(this, _WorkPoller_consumed, "f")) {
          throw new AnthropicError("Cannot iterate over a consumed WorkPoller");
        }
        __classPrivateFieldSet(this, _WorkPoller_consumed, true, "f");
        const log = loggerFor(this.client);
        log.info("poller starting", {
          component: "work-poller",
          environment_id: this.environmentId
        });
        try {
          let attempt = 0;
          while (!__classPrivateFieldGet(this, _WorkPoller_controller, "f").signal.aborted) {
            let work;
            try {
              work = await __classPrivateFieldGet(this, _WorkPoller_runnerClient, "f").beta.environments.work.poll(this.environmentId, {
                "Anthropic-Worker-ID": this.workerId,
                ...__classPrivateFieldGet(this, _WorkPoller_blockMs, "f") !== null ? { block_ms: __classPrivateFieldGet(this, _WorkPoller_blockMs, "f") } : {},
                ...__classPrivateFieldGet(this, _WorkPoller_reclaimOlderThanMs, "f") !== null ? { reclaim_older_than_ms: __classPrivateFieldGet(this, _WorkPoller_reclaimOlderThanMs, "f") } : {}
              }, { headers: buildHeaders([__classPrivateFieldGet(this, _WorkPoller_requestOpts, "f")?.headers]), signal: __classPrivateFieldGet(this, _WorkPoller_controller, "f").signal });
            } catch (e) {
              if (__classPrivateFieldGet(this, _WorkPoller_controller, "f").signal.aborted)
                return;
              if (isFatal4xx(e)) {
                log.error("poll failed permanently, stopping poller", { error: String(e) });
                throw e;
              }
              const wait = applyJitter(backoff2(attempt));
              log.warn("poll failed, backing off", { error: String(e), backoff_ms: wait });
              attempt++;
              await sleep(wait, __classPrivateFieldGet(this, _WorkPoller_controller, "f").signal);
              continue;
            }
            attempt = 0;
            if (work == null) {
              if (__classPrivateFieldGet(this, _WorkPoller_drain, "f"))
                return;
              await sleep(jitter(1e3, 3e3), __classPrivateFieldGet(this, _WorkPoller_controller, "f").signal);
              continue;
            }
            log.info("claimed work", {
              component: "work-poller",
              environment_id: this.environmentId,
              work_id: work.id,
              work_type: work.data.type
            });
            try {
              await __classPrivateFieldGet(this, _WorkPoller_runnerClient, "f").beta.environments.work.ack(work.id, { environment_id: work.environment_id }, { headers: buildHeaders([__classPrivateFieldGet(this, _WorkPoller_requestOpts, "f")?.headers]), signal: __classPrivateFieldGet(this, _WorkPoller_controller, "f").signal });
            } catch (e) {
              log.error("ack failed", { work_id: work.id, error: String(e) });
              continue;
            }
            try {
              yield work;
            } finally {
              if (__classPrivateFieldGet(this, _WorkPoller_autoStop, "f")) {
                try {
                  await __classPrivateFieldGet(this, _WorkPoller_runnerClient, "f").beta.environments.work.stop(work.id, { environment_id: work.environment_id }, { headers: buildHeaders([__classPrivateFieldGet(this, _WorkPoller_requestOpts, "f")?.headers]) });
                } catch (e) {
                  if (!isStatus(e, 409))
                    log.warn("stop failed", { work_id: work.id, error: String(e) });
                }
              }
            }
          }
        } finally {
          __classPrivateFieldGet(this, _WorkPoller_detachExternal, "f").call(this);
        }
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/async-queue.mjs
var _AsyncQueue_items, _AsyncQueue_waiters, _AsyncQueue_closed, AsyncQueue;
var init_async_queue = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/async-queue.mjs"() {
    init_tslib();
    AsyncQueue = class {
      constructor() {
        _AsyncQueue_items.set(this, []);
        _AsyncQueue_waiters.set(this, []);
        _AsyncQueue_closed.set(this, false);
      }
      /** Enqueue an item, or hand it directly to a waiting reader. Returns `false` once closed. */
      push(item) {
        if (__classPrivateFieldGet(this, _AsyncQueue_closed, "f"))
          return false;
        const w = __classPrivateFieldGet(this, _AsyncQueue_waiters, "f").shift();
        if (w)
          w({ done: false, value: item });
        else
          __classPrivateFieldGet(this, _AsyncQueue_items, "f").push(item);
        return true;
      }
      /** Mark the queue done. Idempotent; wakes every pending reader with `done: true`. */
      close() {
        if (__classPrivateFieldGet(this, _AsyncQueue_closed, "f"))
          return;
        __classPrivateFieldSet(this, _AsyncQueue_closed, true, "f");
        while (__classPrivateFieldGet(this, _AsyncQueue_waiters, "f").length > 0) {
          const w = __classPrivateFieldGet(this, _AsyncQueue_waiters, "f").shift();
          w({ done: true, value: void 0 });
        }
      }
      /**
       * Resolve with the next item, or `done: true` once the queue is closed and
       * drained. When `signal` is supplied, aborting it resolves a pending read
       * with `done: true` (cancellation is pushed down here rather than handled by
       * an outer `Promise.race`).
       */
      next(signal) {
        if (__classPrivateFieldGet(this, _AsyncQueue_items, "f").length > 0) {
          return Promise.resolve({ done: false, value: __classPrivateFieldGet(this, _AsyncQueue_items, "f").shift() });
        }
        if (__classPrivateFieldGet(this, _AsyncQueue_closed, "f") || signal?.aborted) {
          return Promise.resolve({ done: true, value: void 0 });
        }
        return new Promise((resolve4) => {
          const waiter = (r) => {
            signal?.removeEventListener("abort", onAbort);
            resolve4(r);
          };
          const onAbort = () => {
            const idx = __classPrivateFieldGet(this, _AsyncQueue_waiters, "f").indexOf(waiter);
            if (idx >= 0)
              __classPrivateFieldGet(this, _AsyncQueue_waiters, "f").splice(idx, 1);
            resolve4({ done: true, value: void 0 });
          };
          __classPrivateFieldGet(this, _AsyncQueue_waiters, "f").push(waiter);
          signal?.addEventListener("abort", onAbort, { once: true });
        });
      }
      /** Synchronously remove and return the next buffered item, or `undefined` if empty. */
      tryShift() {
        return __classPrivateFieldGet(this, _AsyncQueue_items, "f").shift();
      }
    };
    _AsyncQueue_items = /* @__PURE__ */ new WeakMap(), _AsyncQueue_waiters = /* @__PURE__ */ new WeakMap(), _AsyncQueue_closed = /* @__PURE__ */ new WeakMap();
  }
});

// node_modules/@anthropic-ai/sdk/lib/tools/ToolError.mjs
var ToolError;
var init_ToolError = __esm({
  "node_modules/@anthropic-ai/sdk/lib/tools/ToolError.mjs"() {
    ToolError = class extends Error {
      constructor(content) {
        const message = typeof content === "string" ? content : content.map((block) => {
          if (block.type === "text")
            return block.text;
          return `[${block.type}]`;
        }).join(" ");
        super(message);
        this.name = "ToolError";
        this.content = content;
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/lib/tools/BetaRunnableTool.mjs
function toolName(tool) {
  return "name" in tool ? tool.name : tool.mcp_server_name;
}
function toolErrorContent(e) {
  return e instanceof ToolError ? e.content : `Error: ${e instanceof Error ? e.message : String(e)}`;
}
async function runRunnableTool(tool, rawInput, context) {
  try {
    const input = tool.parse ? tool.parse(rawInput) : rawInput;
    const content = await tool.run(input, context);
    return { content, isError: false };
  } catch (e) {
    return { content: toolErrorContent(e), isError: true };
  }
}
var init_BetaRunnableTool = __esm({
  "node_modules/@anthropic-ai/sdk/lib/tools/BetaRunnableTool.mjs"() {
    init_ToolError();
  }
});

// node_modules/@anthropic-ai/sdk/lib/tools/SessionToolRunner.mjs
function isEndTurnIdle(ev) {
  return ev.type === "session.status_idle" && ev.stop_reason?.type === "end_turn";
}
function buildResultEvent(ev, isError, content) {
  if (ev.type === "agent.custom_tool_use") {
    return { type: "user.custom_tool_result", custom_tool_use_id: ev.id, is_error: isError, content };
  }
  return { type: "user.tool_result", tool_use_id: ev.id, is_error: isError, content };
}
function toSessionContent(content) {
  if (typeof content === "string")
    return [{ type: "text", text: content || "(no output)" }];
  const out = content.map((b) => {
    if (b.type === "text")
      return { type: "text", text: b.text || "(no output)" };
    if (b.type === "image" || b.type === "document")
      return b;
    if (b.type === "search_result") {
      return {
        type: "search_result",
        source: b.source,
        title: b.title,
        content: b.content.map((c) => ({ type: "text", text: c.text })),
        citations: { enabled: b.citations?.enabled ?? false }
      };
    }
    return { type: "text", text: JSON.stringify(b) };
  });
  return out.length > 0 ? out : [{ type: "text", text: "(no output)" }];
}
var _IdleClock_maxIdleMs, _IdleClock_onExpire, _IdleClock_blockers, _IdleClock_armPending, _IdleClock_timer, _SessionToolRunner_instances, _SessionToolRunner_consumed, _SessionToolRunner_controller, _SessionToolRunner_detachExternal, _SessionToolRunner_requestOpts, _SessionToolRunner_toolByName, _SessionToolRunner_logger, _SessionToolRunner_seen, _SessionToolRunner_answered, _SessionToolRunner_confirmationVerdicts, _SessionToolRunner_awaitingConfirmation, _SessionToolRunner_results, _SessionToolRunner_inFlightCount, _SessionToolRunner_onIdle, _SessionToolRunner_idleClock, _SessionToolRunner_requestOptions, _SessionToolRunner_streamLoop, _SessionToolRunner_reconcile, _SessionToolRunner_ingestHistory, _SessionToolRunner_handleStreamEvent, _SessionToolRunner_routeToolEvent, _SessionToolRunner_noteConfirmation, _SessionToolRunner_applyVerdict, _SessionToolRunner_surfaceCall, _SessionToolRunner_execute, _SessionToolRunner_sendResult, _SessionToolRunner_drain, STREAM_BACKOFF_START_MS, STREAM_BACKOFF_CAP_MS, TOOL_TIMEOUT_MS, DRAIN_TIMEOUT_MS, SEND_RETRIES, DEFAULT_MAX_IDLE_MS, IdleClock, SessionToolRunner;
var init_SessionToolRunner = __esm({
  "node_modules/@anthropic-ai/sdk/lib/tools/SessionToolRunner.mjs"() {
    init_tslib();
    init_error();
    init_log();
    init_sleep();
    init_backoff();
    init_abort();
    init_async_queue();
    init_headers();
    init_stainless_helper_header();
    init_BetaRunnableTool();
    STREAM_BACKOFF_START_MS = 500;
    STREAM_BACKOFF_CAP_MS = 1e4;
    TOOL_TIMEOUT_MS = 12e4;
    DRAIN_TIMEOUT_MS = 3e4;
    SEND_RETRIES = 3;
    DEFAULT_MAX_IDLE_MS = 6e4;
    IdleClock = class {
      constructor(maxIdleMs, onExpire) {
        _IdleClock_maxIdleMs.set(this, void 0);
        _IdleClock_onExpire.set(this, void 0);
        _IdleClock_blockers.set(this, /* @__PURE__ */ new Set());
        _IdleClock_armPending.set(this, false);
        _IdleClock_timer.set(this, void 0);
        __classPrivateFieldSet(this, _IdleClock_maxIdleMs, maxIdleMs, "f");
        __classPrivateFieldSet(this, _IdleClock_onExpire, onExpire, "f");
      }
      /**
       * Arm on `status_idle{end_turn}`; disarm otherwise. `user.tool_confirmation`
       * is neutral: it signals neither agent activity nor an idle, and its effect
       * on the clock flows through {@link block} / {@link unblock} instead —
       * disarming here would discard the pending arm the verdict is about to
       * settle.
       */
      noteEvent(ev) {
        if (ev.type === "user.tool_confirmation")
          return;
        if (isEndTurnIdle(ev))
          this.arm();
        else
          this.disarm();
      }
      /** Register gated work that must resolve before an idle countdown starts. */
      block(toolUseId) {
        __classPrivateFieldGet(this, _IdleClock_blockers, "f").add(toolUseId);
        if (__classPrivateFieldGet(this, _IdleClock_timer, "f") !== void 0) {
          __classPrivateFieldSet(this, _IdleClock_armPending, true, "f");
          clearTimeout(__classPrivateFieldGet(this, _IdleClock_timer, "f"));
          __classPrivateFieldSet(this, _IdleClock_timer, void 0, "f");
        }
      }
      /**
       * Retire gated work (a no-op for ids never blocked); applies a pending arm —
       * with a fresh full `maxIdleMs` window — once the last blocker retires.
       */
      unblock(toolUseId) {
        __classPrivateFieldGet(this, _IdleClock_blockers, "f").delete(toolUseId);
        if (__classPrivateFieldGet(this, _IdleClock_blockers, "f").size === 0 && __classPrivateFieldGet(this, _IdleClock_armPending, "f"))
          this.arm();
      }
      /**
       * (Re)start the idle countdown — or, while blockers are outstanding, hold
       * the arm pending instead. Stopping then would drop a held call when its
       * verdict later arrives, or cut the runner off before a released call's
       * result can drive the next turn.
       */
      arm() {
        if (__classPrivateFieldGet(this, _IdleClock_maxIdleMs, "f") <= 0)
          return;
        if (__classPrivateFieldGet(this, _IdleClock_blockers, "f").size > 0) {
          __classPrivateFieldSet(this, _IdleClock_armPending, true, "f");
          return;
        }
        __classPrivateFieldSet(this, _IdleClock_armPending, false, "f");
        if (__classPrivateFieldGet(this, _IdleClock_timer, "f") !== void 0)
          clearTimeout(__classPrivateFieldGet(this, _IdleClock_timer, "f"));
        __classPrivateFieldSet(this, _IdleClock_timer, setTimeout(__classPrivateFieldGet(this, _IdleClock_onExpire, "f"), __classPrivateFieldGet(this, _IdleClock_maxIdleMs, "f")), "f");
      }
      /**
       * Cancel the idle countdown and any pending arm. Blockers persist — they
       * track real outstanding work, retired only by {@link unblock}.
       */
      disarm() {
        __classPrivateFieldSet(this, _IdleClock_armPending, false, "f");
        if (__classPrivateFieldGet(this, _IdleClock_timer, "f") !== void 0) {
          clearTimeout(__classPrivateFieldGet(this, _IdleClock_timer, "f"));
          __classPrivateFieldSet(this, _IdleClock_timer, void 0, "f");
        }
      }
    };
    _IdleClock_maxIdleMs = /* @__PURE__ */ new WeakMap(), _IdleClock_onExpire = /* @__PURE__ */ new WeakMap(), _IdleClock_blockers = /* @__PURE__ */ new WeakMap(), _IdleClock_armPending = /* @__PURE__ */ new WeakMap(), _IdleClock_timer = /* @__PURE__ */ new WeakMap();
    SessionToolRunner = class {
      constructor(sessionId, opts) {
        _SessionToolRunner_instances.add(this);
        _SessionToolRunner_consumed.set(this, false);
        _SessionToolRunner_controller.set(this, void 0);
        _SessionToolRunner_detachExternal.set(this, void 0);
        _SessionToolRunner_requestOpts.set(this, void 0);
        _SessionToolRunner_toolByName.set(this, void 0);
        _SessionToolRunner_logger.set(this, void 0);
        _SessionToolRunner_seen.set(this, /* @__PURE__ */ new Set());
        _SessionToolRunner_answered.set(this, /* @__PURE__ */ new Set());
        _SessionToolRunner_confirmationVerdicts.set(this, /* @__PURE__ */ new Map());
        _SessionToolRunner_awaitingConfirmation.set(this, /* @__PURE__ */ new Map());
        _SessionToolRunner_results.set(this, new AsyncQueue());
        _SessionToolRunner_inFlightCount.set(this, 0);
        _SessionToolRunner_onIdle.set(this, null);
        _SessionToolRunner_idleClock.set(this, void 0);
        this.client = opts.client;
        this.sessionId = sessionId;
        this.tools = opts.tools;
        this.maxIdleMs = opts.maxIdleMs ?? DEFAULT_MAX_IDLE_MS;
        __classPrivateFieldSet(this, _SessionToolRunner_logger, loggerFor(opts.client), "f");
        __classPrivateFieldSet(this, _SessionToolRunner_toolByName, new Map(opts.tools.map((t) => [toolName(t), t])), "f");
        __classPrivateFieldSet(this, _SessionToolRunner_controller, new AbortController(), "f");
        __classPrivateFieldSet(this, _SessionToolRunner_detachExternal, linkAbort(opts.signal, __classPrivateFieldGet(this, _SessionToolRunner_controller, "f")), "f");
        __classPrivateFieldSet(this, _SessionToolRunner_requestOpts, opts.requestOptions, "f");
        __classPrivateFieldSet(this, _SessionToolRunner_idleClock, new IdleClock(this.maxIdleMs, () => {
          __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("session idle after end_turn; stopping", {
            component: "session-tool-runner",
            session_id: this.sessionId,
            max_idle_ms: this.maxIdleMs
          });
          __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").abort();
        }), "f");
      }
      /** Read-only view of this runner's abort signal. */
      get signal() {
        return __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").signal;
      }
      /** Abort the runner. Background tasks will wind down and `for await` will exit cleanly. */
      abort() {
        __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").abort();
      }
      async *[(_SessionToolRunner_consumed = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_controller = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_detachExternal = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_requestOpts = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_toolByName = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_logger = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_seen = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_answered = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_confirmationVerdicts = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_awaitingConfirmation = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_results = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_inFlightCount = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_onIdle = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_idleClock = /* @__PURE__ */ new WeakMap(), _SessionToolRunner_instances = /* @__PURE__ */ new WeakSet(), Symbol.asyncIterator)]() {
        if (__classPrivateFieldGet(this, _SessionToolRunner_consumed, "f")) {
          throw new AnthropicError("Cannot iterate over a consumed SessionToolRunner");
        }
        __classPrivateFieldSet(this, _SessionToolRunner_consumed, true, "f");
        __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("session tool runner starting", {
          component: "session-tool-runner",
          session_id: this.sessionId
        });
        const streamPromise = __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_streamLoop).call(this).catch((e) => {
          if (!__classPrivateFieldGet(this, _SessionToolRunner_controller, "f").signal.aborted) {
            __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").error("stream loop failed", { error: String(e) });
          }
          __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").abort();
        });
        try {
          while (true) {
            const next = await __classPrivateFieldGet(this, _SessionToolRunner_results, "f").next(__classPrivateFieldGet(this, _SessionToolRunner_controller, "f").signal);
            if (next.done)
              break;
            yield next.value;
          }
          await streamPromise;
          let pending;
          while ((pending = __classPrivateFieldGet(this, _SessionToolRunner_results, "f").tryShift()) !== void 0) {
            yield pending;
          }
        } finally {
          __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").abort();
          __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").disarm();
          await streamPromise;
          try {
            await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_drain).call(this);
          } catch (e) {
            __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").warn("drain failed", { error: String(e) });
          }
          __classPrivateFieldGet(this, _SessionToolRunner_results, "f").close();
          for (const t of this.tools) {
            try {
              await t.close?.();
            } catch (e) {
              __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").warn("tool.close failed", { tool: toolName(t), error: String(e) });
            }
          }
          __classPrivateFieldGet(this, _SessionToolRunner_detachExternal, "f").call(this);
        }
      }
    };
    _SessionToolRunner_requestOptions = function _SessionToolRunner_requestOptions2() {
      return {
        ...__classPrivateFieldGet(this, _SessionToolRunner_requestOpts, "f"),
        headers: buildHeaders([helperHeader("session-tool-runner"), __classPrivateFieldGet(this, _SessionToolRunner_requestOpts, "f")?.headers]),
        signal: __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").signal
      };
    }, _SessionToolRunner_streamLoop = // ===== event stream =====
    async function _SessionToolRunner_streamLoop2() {
      const ctrl = __classPrivateFieldGet(this, _SessionToolRunner_controller, "f");
      let backoff3 = STREAM_BACKOFF_START_MS;
      while (!ctrl.signal.aborted) {
        try {
          const stream = await this.client.beta.sessions.events.stream(this.sessionId, {}, __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_requestOptions).call(this));
          await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_reconcile).call(this);
          for await (const ev of stream) {
            backoff3 = STREAM_BACKOFF_START_MS;
            if (await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_handleStreamEvent).call(this, ev))
              return;
          }
        } catch (e) {
          ctrl.signal.throwIfAborted();
          if (isFatal4xx(e)) {
            __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").error("permanent stream failure, shutting down", { error: String(e) });
            ctrl.abort();
            throw e;
          }
          __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").warn("stream disconnected, reconnecting", {
            error: String(e),
            backoff_ms: backoff3
          });
        }
        ctrl.signal.throwIfAborted();
        await sleep(backoff3, ctrl.signal);
        backoff3 = Math.min(backoff3 * 2, STREAM_BACKOFF_CAP_MS);
      }
    }, _SessionToolRunner_reconcile = /**
     * Read full history before dispatching so a `tool_use` whose result appears
     * later in the same history is not re-executed. Runs after the live stream is
     * already attached (see {@link SessionToolRunner.#streamLoop}).
     */
    async function _SessionToolRunner_reconcile2() {
      const ctrl = __classPrivateFieldGet(this, _SessionToolRunner_controller, "f");
      const pending = [];
      let lastWasEndTurn = false;
      try {
        for await (const ev of this.client.beta.sessions.events.list(this.sessionId, { limit: 1e3 }, __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_requestOptions).call(this))) {
          __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_ingestHistory).call(this, ev, pending);
          lastWasEndTurn = isEndTurnIdle(ev);
        }
      } catch (e) {
        ctrl.signal.throwIfAborted();
        __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").warn("reconcile list failed", { error: String(e) });
        for (const ev of pending)
          __classPrivateFieldGet(this, _SessionToolRunner_seen, "f").delete(ev.id);
        return;
      }
      const unanswered = pending.filter((ev) => !__classPrivateFieldGet(this, _SessionToolRunner_answered, "f").has(ev.id));
      __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").disarm();
      for (const ev of unanswered)
        await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_routeToolEvent).call(this, ev);
      for (const held of [...__classPrivateFieldGet(this, _SessionToolRunner_awaitingConfirmation, "f").values()]) {
        const verdict = __classPrivateFieldGet(this, _SessionToolRunner_confirmationVerdicts, "f").get(held.id);
        if (verdict !== void 0)
          await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_applyVerdict).call(this, held, verdict);
      }
      const outstanding = unanswered.filter((ev) => !__classPrivateFieldGet(this, _SessionToolRunner_answered, "f").has(ev.id) && !__classPrivateFieldGet(this, _SessionToolRunner_awaitingConfirmation, "f").has(ev.id));
      if (lastWasEndTurn && outstanding.length === 0)
        __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").arm();
      else
        __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").disarm();
    }, _SessionToolRunner_ingestHistory = function _SessionToolRunner_ingestHistory2(ev, pending) {
      if (ev.type === "agent.tool_use" || ev.type === "agent.custom_tool_use") {
        __classPrivateFieldGet(this, _SessionToolRunner_seen, "f").add(ev.id);
        if (!__classPrivateFieldGet(this, _SessionToolRunner_answered, "f").has(ev.id))
          pending.push(ev);
      } else if (ev.type === "user.tool_result") {
        __classPrivateFieldGet(this, _SessionToolRunner_answered, "f").add(ev.tool_use_id);
      } else if (ev.type === "user.custom_tool_result") {
        __classPrivateFieldGet(this, _SessionToolRunner_answered, "f").add(ev.custom_tool_use_id);
      } else if (ev.type === "user.tool_confirmation") {
        if (!__classPrivateFieldGet(this, _SessionToolRunner_answered, "f").has(ev.tool_use_id))
          __classPrivateFieldGet(this, _SessionToolRunner_confirmationVerdicts, "f").set(ev.tool_use_id, ev.result);
      }
    }, _SessionToolRunner_handleStreamEvent = /** Returns true when the runner should exit. */
    async function _SessionToolRunner_handleStreamEvent2(ev) {
      __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").noteEvent(ev);
      switch (ev.type) {
        case "agent.tool_use":
        case "agent.custom_tool_use":
          if (!__classPrivateFieldGet(this, _SessionToolRunner_seen, "f").has(ev.id)) {
            __classPrivateFieldGet(this, _SessionToolRunner_seen, "f").add(ev.id);
            await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_routeToolEvent).call(this, ev);
          }
          return false;
        case "user.tool_confirmation":
          await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_noteConfirmation).call(this, ev);
          return false;
        case "user.tool_result":
          __classPrivateFieldGet(this, _SessionToolRunner_answered, "f").add(ev.tool_use_id);
          return false;
        case "user.custom_tool_result":
          __classPrivateFieldGet(this, _SessionToolRunner_answered, "f").add(ev.custom_tool_use_id);
          return false;
        case "session.status_terminated":
        case "session.deleted":
          __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("session terminated", {
            component: "session-tool-runner",
            session_id: this.sessionId
          });
          __classPrivateFieldGet(this, _SessionToolRunner_controller, "f").abort();
          return true;
        default:
          return false;
      }
    }, _SessionToolRunner_routeToolEvent = // ===== confirmation gating (always_ask tools) =====
    /**
     * Dispatch `ev`, honoring its evaluated permission. A call the server gated
     * (`evaluated_permission == "ask"`) is held until its `user.tool_confirmation`
     * arrives. Fails closed: only an explicit `allow` verdict releases a gated
     * call; a server-side `deny` overrides any recorded verdict; an unrecognized
     * permission is held like `ask` and an unrecognized verdict is denied.
     */
    async function _SessionToolRunner_routeToolEvent2(ev) {
      const permission = ev.evaluated_permission;
      const verdict = permission === "deny" ? "deny" : __classPrivateFieldGet(this, _SessionToolRunner_confirmationVerdicts, "f").get(ev.id);
      if (verdict === void 0) {
        if (permission === void 0 || permission === "allow") {
          await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_execute).call(this, ev, void 0);
        } else if (!__classPrivateFieldGet(this, _SessionToolRunner_awaitingConfirmation, "f").has(ev.id)) {
          __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("tool call awaiting confirmation; holding", {
            component: "session-tool-runner",
            session_id: this.sessionId,
            tool: ev.name,
            tool_use_id: ev.id
          });
          __classPrivateFieldGet(this, _SessionToolRunner_awaitingConfirmation, "f").set(ev.id, ev);
          __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").block(ev.id);
        }
        return;
      }
      await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_applyVerdict).call(this, ev, verdict);
    }, _SessionToolRunner_noteConfirmation = /** Record an allow/deny verdict and release the held call it gates, if any. */
    async function _SessionToolRunner_noteConfirmation2(ev) {
      __classPrivateFieldGet(this, _SessionToolRunner_confirmationVerdicts, "f").set(ev.tool_use_id, ev.result);
      const held = __classPrivateFieldGet(this, _SessionToolRunner_awaitingConfirmation, "f").get(ev.tool_use_id);
      if (held === void 0)
        return;
      await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_applyVerdict).call(this, held, ev.result);
    }, _SessionToolRunner_applyVerdict = /**
     * Dispatch or resolve a gated call according to its verdict.
     *
     * The idle-clock blocker accounting lives here: a denial retires the held
     * call's blocker, while an allow keeps one on the call — taking it now if the
     * verdict was already known when the call was routed, so it was never held —
     * until `#execute` has finished with it. The countdown must not run over
     * gated work that is still in flight.
     */
    async function _SessionToolRunner_applyVerdict2(ev, verdict) {
      const wasHeld = __classPrivateFieldGet(this, _SessionToolRunner_awaitingConfirmation, "f").delete(ev.id);
      if (verdict === "allow") {
        __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("tool call confirmed", {
          component: "session-tool-runner",
          session_id: this.sessionId,
          tool: ev.name,
          tool_use_id: ev.id
        });
        if (!wasHeld)
          __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").block(ev.id);
        try {
          await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_execute).call(this, ev, "allow");
        } finally {
          __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").unblock(ev.id);
        }
        return;
      }
      if (wasHeld)
        __classPrivateFieldGet(this, _SessionToolRunner_idleClock, "f").unblock(ev.id);
      __classPrivateFieldGet(this, _SessionToolRunner_answered, "f").add(ev.id);
      __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("tool call denied; not executing", {
        component: "session-tool-runner",
        session_id: this.sessionId,
        tool: ev.name,
        tool_use_id: ev.id
      });
      __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_surfaceCall).call(this, {
        event: ev,
        toolUseId: ev.id,
        name: ev.name,
        isError: false,
        posted: false,
        confirmation: "deny"
      });
    }, _SessionToolRunner_surfaceCall = function _SessionToolRunner_surfaceCall2(call) {
      __classPrivateFieldGet(this, _SessionToolRunner_results, "f").push(call);
    }, _SessionToolRunner_execute = // ===== tool execution =====
    async function _SessionToolRunner_execute2(ev, confirmation) {
      var _a2, _b;
      if (__classPrivateFieldGet(this, _SessionToolRunner_answered, "f").has(ev.id))
        return;
      __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("executing tool", {
        component: "session-tool-runner",
        session_id: this.sessionId,
        tool: ev.name,
        tool_use_id: ev.id
      });
      __classPrivateFieldSet(this, _SessionToolRunner_inFlightCount, (_a2 = __classPrivateFieldGet(this, _SessionToolRunner_inFlightCount, "f"), _a2++, _a2), "f");
      try {
        const tool = __classPrivateFieldGet(this, _SessionToolRunner_toolByName, "f").get(ev.name);
        if (!tool) {
          __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").info("tool not owned by this runner; leaving the tool_use_id pending for its owner", {
            component: "session-tool-runner",
            session_id: this.sessionId,
            tool: ev.name,
            tool_use_id: ev.id
          });
          __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_surfaceCall).call(this, {
            event: ev,
            toolUseId: ev.id,
            name: ev.name,
            isError: false,
            posted: false,
            confirmation
          });
          return;
        }
        let content;
        let isError;
        const toolCtrl = new AbortController();
        const detachTool = linkAbort(__classPrivateFieldGet(this, _SessionToolRunner_controller, "f").signal, toolCtrl);
        const timer = setTimeout(() => toolCtrl.abort(), TOOL_TIMEOUT_MS);
        try {
          const outcome = await runRunnableTool(tool, ev.input, {
            toolUse: ev,
            toolUseBlock: ev,
            signal: toolCtrl.signal
          });
          content = outcome.content;
          isError = outcome.isError;
        } finally {
          clearTimeout(timer);
          detachTool();
        }
        const result = buildResultEvent(ev, isError, toSessionContent(content));
        const posted = await __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_sendResult).call(this, result, ev.id);
        __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_surfaceCall).call(this, {
          event: ev,
          result,
          toolUseId: ev.id,
          name: ev.name,
          isError,
          posted,
          confirmation
        });
      } finally {
        __classPrivateFieldSet(this, _SessionToolRunner_inFlightCount, (_b = __classPrivateFieldGet(this, _SessionToolRunner_inFlightCount, "f"), _b--, _b), "f");
        if (__classPrivateFieldGet(this, _SessionToolRunner_inFlightCount, "f") === 0)
          __classPrivateFieldGet(this, _SessionToolRunner_onIdle, "f")?.call(this);
      }
    }, _SessionToolRunner_sendResult = async function _SessionToolRunner_sendResult2(result, toolUseId) {
      const ctrl = __classPrivateFieldGet(this, _SessionToolRunner_controller, "f");
      let lastErr;
      for (let i = 0; i < SEND_RETRIES; i++) {
        ctrl.signal.throwIfAborted();
        try {
          await this.client.beta.sessions.events.send(this.sessionId, { events: [result] }, __classPrivateFieldGet(this, _SessionToolRunner_instances, "m", _SessionToolRunner_requestOptions).call(this));
          __classPrivateFieldGet(this, _SessionToolRunner_answered, "f").add(toolUseId);
          return true;
        } catch (e) {
          lastErr = e;
          if (isFatal4xx(e))
            break;
          if (i < SEND_RETRIES - 1)
            await sleep((i + 1) * 1e3, ctrl.signal);
        }
      }
      __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").error("failed to send tool result", {
        tool_use_id: toolUseId,
        error: String(lastErr)
      });
      return false;
    }, _SessionToolRunner_drain = /** Wait (bounded) for in-flight tool executions to finish during teardown. */
    async function _SessionToolRunner_drain2() {
      if (__classPrivateFieldGet(this, _SessionToolRunner_inFlightCount, "f") === 0)
        return;
      await Promise.race([new Promise((r) => __classPrivateFieldSet(this, _SessionToolRunner_onIdle, r, "f")), sleep(DRAIN_TIMEOUT_MS)]);
      __classPrivateFieldSet(this, _SessionToolRunner_onIdle, null, "f");
      if (__classPrivateFieldGet(this, _SessionToolRunner_inFlightCount, "f") > 0) {
        __classPrivateFieldGet(this, _SessionToolRunner_logger, "f").warn("drain timeout exceeded");
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/lib/transform-json-schema.mjs
var init_transform_json_schema = __esm({
  "node_modules/@anthropic-ai/sdk/lib/transform-json-schema.mjs"() {
    init_utils2();
  }
});

// node_modules/@anthropic-ai/sdk/helpers/beta/json-schema.mjs
function betaTool(options) {
  if (options.inputSchema.type !== "object") {
    throw new Error(`JSON schema for tool "${options.name}" must be an object, but got ${options.inputSchema.type}`);
  }
  return {
    type: "custom",
    name: options.name,
    input_schema: options.inputSchema,
    description: options.description,
    run: options.run,
    parse: (content) => content,
    ...options.close ? { close: options.close } : {}
  };
}
var init_json_schema = __esm({
  "node_modules/@anthropic-ai/sdk/helpers/beta/json-schema.mjs"() {
    init_sdk();
    init_transform_json_schema();
  }
});

// node_modules/@anthropic-ai/sdk/internal/utils/promise.mjs
function promiseWithResolvers() {
  let resolve4;
  let reject;
  const promise = new Promise((res, rej) => {
    resolve4 = res;
    reject = rej;
  });
  return { promise, resolve: resolve4, reject };
}
var init_promise = __esm({
  "node_modules/@anthropic-ai/sdk/internal/utils/promise.mjs"() {
  }
});

// node_modules/@anthropic-ai/sdk/tools/agent-toolset/fs-util.mjs
import * as fs from "node:fs/promises";
import * as path2 from "node:path";
import { randomUUID } from "node:crypto";
async function realpathOrSelf(p) {
  try {
    return await fs.realpath(p);
  } catch {
    return p;
  }
}
async function canonicalize(abs) {
  const tail = [];
  let prefix = abs;
  let hops = 0;
  for (; ; ) {
    let real;
    try {
      real = await fs.realpath(prefix);
    } catch {
      let isLink = false;
      try {
        isLink = (await fs.lstat(prefix)).isSymbolicLink();
      } catch {
      }
      if (isLink) {
        if (++hops > 40) {
          throw new ToolError(`path ${JSON.stringify(abs)} has too many levels of symbolic links`);
        }
        prefix = path2.resolve(path2.dirname(prefix), await fs.readlink(prefix));
        continue;
      }
      const parent = path2.dirname(prefix);
      if (parent === prefix)
        return abs;
      tail.push(path2.basename(prefix));
      prefix = parent;
      continue;
    }
    return tail.length ? path2.join(real, ...tail.reverse()) : real;
  }
}
async function confineToRoot(root, p, opts) {
  const allowOutside = opts?.allowOutside ?? false;
  const realRoot = await realpathOrSelf(path2.resolve(root));
  const abs = path2.resolve(realRoot, p);
  if (allowOutside)
    return abs;
  const real = await canonicalize(abs);
  if (real !== realRoot && !real.startsWith(realRoot + path2.sep)) {
    throw new ToolError(`path ${JSON.stringify(p)} escapes workdir`);
  }
  return real;
}
async function atomicWriteFile(targetPath, content) {
  const dir = path2.dirname(targetPath);
  const tempPath = path2.join(dir, `.tmp-${process.pid}-${randomUUID()}`);
  let handle;
  try {
    handle = await fs.open(tempPath, "wx", FILE_CREATE_MODE);
    await handle.writeFile(content, "utf-8");
    await handle.sync();
    await handle.close();
    handle = void 0;
    await fs.rename(tempPath, targetPath);
  } catch (err) {
    if (handle)
      await handle.close().catch(() => {
      });
    await fs.unlink(tempPath).catch(() => {
    });
    throw err;
  }
}
function fsErrorMessage(err, file) {
  const code = err?.code;
  switch (code) {
    case "ENOENT":
      return `${file}: no such file or directory`;
    case "EACCES":
    case "EPERM":
      return `${file}: permission denied`;
    case "ENOTDIR":
      return `${file}: not a directory`;
    case "EISDIR":
      return `${file}: is a directory`;
    case "ELOOP":
      return `${file}: too many levels of symbolic links`;
    case "ENAMETOOLONG":
      return `${file}: file name too long`;
    case "ENOSPC":
      return `${file}: no space left on device`;
    case "EMFILE":
    case "ENFILE":
      return `${file}: too many open files`;
    default:
      return `${file}: ${err instanceof Error ? err.message : String(err)}`;
  }
}
var DIR_CREATE_MODE, FILE_CREATE_MODE;
var init_fs_util = __esm({
  "node_modules/@anthropic-ai/sdk/tools/agent-toolset/fs-util.mjs"() {
    init_ToolError();
    DIR_CREATE_MODE = 493;
    FILE_CREATE_MODE = 420;
  }
});

// node_modules/@anthropic-ai/sdk/tools/agent-toolset/skills.mjs
import * as fs2 from "node:fs/promises";
import * as fssync from "node:fs";
import * as path3 from "node:path";
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
async function setupSkills(ctx) {
  const { client, sessionId } = ctx;
  if (!client || !sessionId)
    return async () => {
    };
  const log = loggerFor(client);
  const session = await client.beta.sessions.retrieve(sessionId);
  const skillsRoot = path3.resolve(ctx.workdir, "skills");
  const created = [];
  for (const skill of session.agent.skills) {
    try {
      const versionId = await resolveSkillVersion(client, skill.skill_id, skill.version);
      const version = await client.beta.skills.versions.retrieve(versionId, { skill_id: skill.skill_id });
      let dirname5 = path3.basename(version.name.trim());
      if (dirname5 === "" || dirname5 === "." || dirname5 === "..")
        dirname5 = skill.skill_id;
      const dest = path3.resolve(skillsRoot, dirname5);
      if (dest !== skillsRoot && !dest.startsWith(skillsRoot + path3.sep)) {
        log.warn("skill name escapes the skills dir; skipping", {
          component: "agent-tool-context",
          name: version.name
        });
        continue;
      }
      const resp = await client.beta.skills.versions.download(versionId, { skill_id: skill.skill_id });
      await fs2.rm(dest, { recursive: true, force: true });
      await fs2.mkdir(dest, { recursive: true, mode: DIR_CREATE_MODE });
      created.push(dest);
      await extractSkillArchive(resp, dest);
      log.info("downloaded skill", {
        component: "agent-tool-context",
        skill_id: skill.skill_id,
        version: versionId,
        dest
      });
    } catch (e) {
      log.warn("failed to download skill", {
        component: "agent-tool-context",
        skill_id: skill.skill_id,
        error: String(e)
      });
    }
  }
  return async () => {
    for (const dest of created) {
      await fs2.rm(dest, { recursive: true, force: true }).catch((e) => {
        log.warn("failed to clean up skill", { component: "agent-tool-context", dest, error: String(e) });
      });
    }
  };
}
async function resolveSkillVersion(client, skillId, version) {
  if (/^\d+$/.test(version))
    return version;
  let newest;
  for await (const v of client.beta.skills.versions.list(skillId)) {
    if (/^\d+$/.test(v.version) && (newest === void 0 || BigInt(v.version) > BigInt(newest))) {
      newest = v.version;
    }
  }
  if (newest === void 0) {
    throw new AnthropicError(`skill ${JSON.stringify(skillId)} has no concrete version to resolve ${JSON.stringify(version)} against`);
  }
  return newest;
}
function assertSafeMemberNames(names) {
  for (const raw of names.split("\n")) {
    const entry = raw.trim();
    if (!entry)
      continue;
    if (path3.isAbsolute(entry) || entry.split(/[\\/]/).includes("..")) {
      throw new AnthropicError(`refusing to extract unsafe archive member: ${entry}`);
    }
  }
}
function assertNoSpecialMembers(verboseListing) {
  for (const line of verboseListing.split("\n")) {
    const type = line.trimStart()[0];
    if (type === "l" || type === "h" || type === "b" || type === "c" || type === "p" || type === "s") {
      throw new AnthropicError("refusing to extract archive with symlink/hardlink/device member");
    }
  }
}
async function runArchiveTool(cmd, args) {
  try {
    const { stdout } = await execFileAsync(cmd, args);
    return stdout;
  } catch (e) {
    if (e != null && typeof e === "object" && e.code === "ENOENT") {
      throw new AnthropicError(`skill extraction requires the \`${cmd}\` command, but it was not found on PATH`);
    }
    throw e;
  }
}
function archiveTopDir(listing) {
  let top;
  let nested = false;
  for (const raw of listing.split("\n")) {
    const parts = raw.trim().split("/").filter((p) => p !== "" && p !== ".");
    if (parts.length === 0)
      continue;
    const first = parts[0];
    if (top === void 0)
      top = first;
    else if (first !== top)
      return "";
    if (parts.length > 1)
      nested = true;
  }
  return top !== void 0 && nested ? top : "";
}
async function extractSkillArchive(resp, dest) {
  const tmp = path3.join(dest, `.skill-archive-${process.pid}-${Date.now()}`);
  if (!resp.body) {
    throw new AnthropicError("skill download response had no body");
  }
  await pipeline(Readable.fromWeb(resp.body), fssync.createWriteStream(tmp));
  const stage = path3.join(path3.dirname(dest), `.skill-stage-${process.pid}-${Date.now()}`);
  try {
    const head = await readHead(tmp, 4);
    const isZip = head.length >= 4 && head[0] === 80 && head[1] === 75 && head[2] === 3 && head[3] === 4;
    const archiveCmd = isZip ? "unzip" : "tar";
    const listing = await runArchiveTool(archiveCmd, isZip ? ["-Z1", tmp] : ["-tf", tmp]);
    assertSafeMemberNames(listing);
    assertNoSpecialMembers(await runArchiveTool(archiveCmd, isZip ? ["-Z", tmp] : ["-tvf", tmp]));
    const top = archiveTopDir(listing);
    await fs2.mkdir(stage, { recursive: true, mode: DIR_CREATE_MODE });
    await runArchiveTool(archiveCmd, isZip ? ["-oq", tmp, "-d", stage] : ["-xf", tmp, "-C", stage]);
    const srcRoot = top ? path3.join(stage, top) : stage;
    for (const entry of await fs2.readdir(srcRoot)) {
      await fs2.rename(path3.join(srcRoot, entry), path3.join(dest, entry));
    }
  } finally {
    await fs2.rm(tmp, { force: true });
    await fs2.rm(stage, { recursive: true, force: true });
  }
}
async function readHead(file, n) {
  const handle = await fs2.open(file, "r");
  try {
    const buf = Buffer.alloc(n);
    const { bytesRead } = await handle.read(buf, 0, n, 0);
    return buf.subarray(0, bytesRead);
  } finally {
    await handle.close();
  }
}
var execFileAsync;
var init_skills = __esm({
  "node_modules/@anthropic-ai/sdk/tools/agent-toolset/skills.mjs"() {
    init_error();
    init_log();
    init_fs_util();
    execFileAsync = promisify(execFile);
  }
});

// node_modules/@anthropic-ai/sdk/tools/agent-toolset/node.mjs
var node_exports = {};
__export(node_exports, {
  BashSession: () => BashSession,
  BashTimeoutError: () => BashTimeoutError,
  betaAgentToolset20260401: () => betaAgentToolset20260401,
  betaBashTool: () => betaBashTool,
  betaEditTool: () => betaEditTool,
  betaGlobTool: () => betaGlobTool,
  betaGrepTool: () => betaGrepTool,
  betaReadTool: () => betaReadTool,
  betaWriteTool: () => betaWriteTool,
  extractSkillArchive: () => extractSkillArchive,
  resolvePath: () => resolvePath,
  resolveSkillVersion: () => resolveSkillVersion,
  setupSkills: () => setupSkills
});
import * as fs3 from "node:fs/promises";
import * as fssync2 from "node:fs";
import * as path4 from "node:path";
import * as cp from "node:child_process";
import * as crypto from "node:crypto";
import * as readline from "node:readline";
function resolveMaxBytes(configured) {
  return configured === void 0 ? DEFAULT_MAX_FILE_BYTES : configured;
}
function betaAgentToolset20260401(ctx) {
  return [
    betaBashTool(ctx),
    betaReadTool(ctx),
    betaWriteTool(ctx),
    betaEditTool(ctx),
    betaGlobTool(ctx),
    betaGrepTool(ctx)
  ];
}
function resolvePath(ctx, p) {
  return confineToRoot(ctx.workdir, p, { allowOutside: ctx.unrestrictedPaths ?? false });
}
function scrubbedShellEnv() {
  const env = {};
  for (const [key, value] of Object.entries(process.env)) {
    if (key.startsWith("ANTHROPIC_"))
      continue;
    env[key] = value;
  }
  return env;
}
function betaBashTool(ctx) {
  let session;
  let tail = Promise.resolve();
  return betaTool({
    name: "bash",
    description: "Run a bash command in a persistent shell. State (cwd, env vars) persists across calls.",
    inputSchema: {
      type: "object",
      properties: {
        command: { type: "string", description: "The command to run" },
        restart: { type: "boolean", description: "Restart the persistent shell before running" },
        timeout_ms: { type: "integer", description: "Per-call timeout in milliseconds" }
      }
    },
    run: async ({ command, restart, timeout_ms }, context) => {
      const prev = tail;
      const gate = promiseWithResolvers();
      tail = gate.promise;
      try {
        await prev;
      } catch {
      }
      try {
        if (restart) {
          session?.close();
          session = void 0;
        }
        if (!command) {
          if (restart)
            return "bash session restarted";
          throw new ToolError("bash: command is required");
        }
        session ?? (session = new BashSession(ctx.workdir, ctx.env));
        try {
          const { output, exitCode } = await session.exec(command, {
            timeoutMs: timeout_ms ?? BASH_DEFAULT_TIMEOUT_MS,
            signal: context?.signal
          });
          if (exitCode !== 0)
            throw new ToolError(output || `exit ${exitCode}`);
          return output;
        } catch (e) {
          if (e instanceof ToolError)
            throw e;
          session.close();
          session = void 0;
          throw new ToolError(`bash: ${e instanceof Error ? e.message : String(e)}`);
        }
      } finally {
        gate.resolve();
      }
    },
    close: () => {
      session?.close();
      session = void 0;
    }
  });
}
function betaReadTool(ctx) {
  return betaTool({
    name: "read",
    description: "Read a UTF-8 text file relative to the workdir.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: { type: "string" },
        view_range: {
          type: "array",
          items: { type: "integer" },
          description: "[start_line, end_line] 1-indexed inclusive"
        }
      },
      required: ["file_path"]
    },
    run: async ({ file_path, view_range }) => {
      if (!file_path)
        throw new ToolError("read: file_path is required");
      const abs = await resolvePath(ctx, file_path);
      let data;
      try {
        const st = await fs3.stat(abs);
        if (!st.isFile()) {
          throw new ToolError(`read: ${file_path} is not a regular file`);
        }
        const limit2 = resolveMaxBytes(ctx.maxFileBytes);
        if (limit2 !== null && st.size > limit2) {
          throw new ToolError(`read: ${file_path} is ${st.size} bytes, exceeds ${limit2}-byte limit. Use bash (head/tail/sed) to read a slice.`);
        }
        data = await fs3.readFile(abs, "utf8");
      } catch (e) {
        if (e instanceof ToolError)
          throw e;
        throw new ToolError(`read: ${fsErrorMessage(e, file_path)}`);
      }
      if (!view_range)
        return data;
      if (view_range.length !== 2)
        throw new ToolError("read: view_range must be [start_line, end_line]");
      const [startLine, endLine] = view_range;
      const lines = data.split("\n");
      const start = Math.max(0, startLine - 1);
      const end = endLine > 0 ? endLine : lines.length;
      return lines.slice(start, end).join("\n");
    }
  });
}
function betaWriteTool(ctx) {
  return betaTool({
    name: "write",
    description: "Write a UTF-8 text file relative to the workdir, creating parent directories as needed.",
    inputSchema: {
      type: "object",
      properties: { file_path: { type: "string" }, content: { type: "string" } },
      required: ["file_path", "content"]
    },
    run: async ({ file_path, content }) => {
      if (!file_path)
        throw new ToolError("write: file_path is required");
      const abs = await resolvePath(ctx, file_path);
      try {
        await fs3.mkdir(path4.dirname(abs), { recursive: true, mode: DIR_CREATE_MODE });
        await atomicWriteFile(abs, content ?? "");
      } catch (e) {
        throw new ToolError(`write: ${fsErrorMessage(e, file_path)}`);
      }
      return `wrote ${Buffer.byteLength(content ?? "")} bytes to ${file_path}`;
    }
  });
}
function betaEditTool(ctx) {
  return betaTool({
    name: "edit",
    description: "Replace old_string with new_string in a file. old_string must be unique unless replace_all.",
    inputSchema: {
      type: "object",
      properties: {
        file_path: { type: "string" },
        old_string: { type: "string" },
        new_string: { type: "string" },
        replace_all: { type: "boolean" }
      },
      required: ["file_path", "old_string", "new_string"]
    },
    run: async ({ file_path, old_string, new_string, replace_all }) => {
      if (!file_path)
        throw new ToolError("edit: file_path is required");
      if (!old_string)
        throw new ToolError("edit: old_string is required");
      const abs = await resolvePath(ctx, file_path);
      let data;
      try {
        const st = await fs3.stat(abs);
        if (!st.isFile()) {
          throw new ToolError(`edit: ${file_path} is not a regular file`);
        }
        const limit2 = resolveMaxBytes(ctx.maxFileBytes);
        if (limit2 !== null && st.size > limit2) {
          throw new ToolError(`edit: ${file_path} is ${st.size} bytes, exceeds ${limit2}-byte limit. Use bash (sed/awk) to edit a large file.`);
        }
        data = await fs3.readFile(abs, "utf8");
      } catch (e) {
        if (e instanceof ToolError)
          throw e;
        throw new ToolError(`edit: ${fsErrorMessage(e, file_path)}`);
      }
      const count = data.split(old_string).length - 1;
      if (count === 0)
        throw new ToolError(`edit: old_string not found in ${file_path}`);
      let updated;
      if (replace_all) {
        updated = data.split(old_string).join(new_string);
      } else {
        if (count > 1)
          throw new ToolError(`edit: old_string appears ${count} times in ${file_path} (must be unique)`);
        updated = data.replace(old_string, () => new_string);
      }
      try {
        await atomicWriteFile(abs, updated);
      } catch (e) {
        throw new ToolError(`edit: write: ${fsErrorMessage(e, file_path)}`);
      }
      return `edited ${file_path} (${replace_all ? count : 1} replacement(s))`;
    }
  });
}
function betaGlobTool(ctx) {
  return betaTool({
    name: "glob",
    description: "Match files under the workdir against a glob pattern. Results are mtime-sorted, newest first.",
    inputSchema: {
      type: "object",
      properties: {
        pattern: { type: "string" },
        path: { type: "string", description: "Directory to search in. Defaults to the workdir." }
      },
      required: ["pattern"]
    },
    run: async ({ pattern, path: searchPath }) => {
      if (!pattern)
        throw new ToolError("glob: pattern is required");
      let root = path4.resolve(ctx.workdir);
      let pat = pattern;
      if (path4.isAbsolute(pattern)) {
        if (!ctx.unrestrictedPaths)
          throw new ToolError("glob: absolute pattern not permitted");
        root = path4.parse(pattern).root;
        pat = path4.relative(root, pattern);
      } else if (searchPath) {
        root = await resolvePath(ctx, searchPath);
      }
      if (!ctx.unrestrictedPaths && pat.split(/[\\/]/).includes("..")) {
        throw new ToolError('glob: ".." is not permitted in the pattern');
      }
      const realRoot = ctx.unrestrictedPaths ? root : await fs3.realpath(root).catch(() => root);
      const matches = [];
      try {
        for await (const entry of fsGlob(pat, {
          cwd: root,
          withFileTypes: true,
          exclude: (d) => d.name === ".git" || d.name === "node_modules"
        })) {
          if (!entry.isFile())
            continue;
          const full = path4.join(entry.parentPath, entry.name);
          if (!ctx.unrestrictedPaths) {
            let real;
            try {
              real = await fs3.realpath(full);
            } catch {
              continue;
            }
            if (!isWithin(realRoot, real))
              continue;
          }
          let mtime = 0;
          try {
            mtime = (await fs3.stat(full)).mtimeMs;
          } catch {
          }
          matches.push({ path: full, mtime });
        }
      } catch (e) {
        throw new ToolError(`glob: ${e instanceof Error ? e.message : String(e)}`);
      }
      if (matches.length === 0)
        return "no matches";
      matches.sort((a, b) => b.mtime - a.mtime);
      return matches.slice(0, GLOB_RESULT_LIMIT).map((m) => m.path).join("\n");
    }
  });
}
function betaGrepTool(ctx) {
  return betaTool({
    name: "grep",
    description: "Search file contents for a regex. Uses ripgrep if available, otherwise a built-in walker.",
    inputSchema: {
      type: "object",
      properties: { pattern: { type: "string" }, path: { type: "string" } },
      required: ["pattern"]
    },
    run: async ({ pattern, path: p }, context) => {
      if (!pattern)
        throw new ToolError("grep: pattern is required");
      let searchPath = path4.resolve(ctx.workdir);
      if (p)
        searchPath = await resolvePath(ctx, p);
      const rg = await findRg();
      return rg ? runRipgrep(rg, pattern, searchPath, context?.signal) : runWalkGrep(pattern, searchPath, context?.signal);
    }
  });
}
function runRipgrep(rg, pattern, searchPath, signal) {
  return new Promise((resolve4, reject) => {
    const proc = cp.spawn(rg, ["-n", "--no-heading", "-e", pattern, "--", searchPath], {
      ...signal ? { signal } : {}
    });
    let out = "";
    let errOut = "";
    let truncated = false;
    proc.stdout.on("data", (d) => {
      if (truncated)
        return;
      out += d;
      if (out.length > GREP_OUTPUT_LIMIT) {
        truncated = true;
        out = out.slice(0, GREP_OUTPUT_LIMIT);
        proc.kill("SIGKILL");
      }
    });
    proc.stderr.on("data", (d) => errOut += d);
    proc.on("close", (code) => {
      if (signal?.aborted)
        return reject(new ToolError("grep: aborted"));
      if (truncated)
        return resolve4(out + `
[output truncated at ${GREP_OUTPUT_LIMIT} bytes]`);
      if (code === 0)
        return resolve4(out);
      if (code === 1)
        return resolve4("no matches");
      reject(new ToolError(`grep: rg failed: ${errOut || `exit ${code}`}`));
    });
    proc.on("error", (e) => {
      if (signal?.aborted)
        return reject(new ToolError("grep: aborted"));
      reject(new ToolError(`grep: rg failed: ${e.message}`));
    });
  });
}
async function runWalkGrep(pattern, root, signal) {
  let re;
  try {
    re = new RegExp(pattern);
  } catch (e) {
    throw new ToolError(`grep: invalid regex: ${e instanceof Error ? e.message : String(e)}`);
  }
  const hits = [];
  let budget = GREP_OUTPUT_LIMIT;
  const push = (line) => {
    budget -= line.length + 1;
    if (budget < 0) {
      hits.push(`[output truncated at ${GREP_OUTPUT_LIMIT} bytes]`);
      return false;
    }
    hits.push(line);
    return true;
  };
  const stat2 = await fs3.stat(root).catch(() => null);
  if (stat2?.isFile()) {
    await grepFile(root, re, push);
  } else {
    await walk(root, "", (rel) => grepFile(path4.join(root, rel), re, push), signal);
  }
  if (signal?.aborted)
    throw new ToolError("grep: aborted");
  if (hits.length === 0)
    return "no matches";
  return hits.join("\n");
}
async function grepFile(file, re, push) {
  const stream = fssync2.createReadStream(file, { encoding: "utf8" });
  const rl = readline.createInterface({ input: stream, crlfDelay: Infinity });
  let i = 0;
  try {
    for await (const line of rl) {
      i++;
      if (line.length > GREP_MAX_LINE_LENGTH)
        continue;
      if (re.test(line) && !push(`${file}:${i}:${line}`))
        return false;
    }
  } catch {
  } finally {
    stream.destroy();
  }
  return true;
}
function isWithin(root, p) {
  const rel = path4.relative(root, p);
  return rel === "" || !rel.startsWith(".." + path4.sep) && rel !== ".." && !path4.isAbsolute(rel);
}
async function walk(root, rel, fn, signal) {
  let remaining = WALK_MAX_ENTRIES;
  async function inner(rel2, depth) {
    if (depth > WALK_MAX_DEPTH)
      return true;
    if (signal?.aborted)
      return false;
    let entries;
    try {
      entries = await fs3.readdir(path4.join(root, rel2), { withFileTypes: true });
    } catch {
      return true;
    }
    for (const e of entries) {
      if (e.name === ".git" || e.name === "node_modules")
        continue;
      if (remaining-- <= 0)
        return false;
      if (signal?.aborted)
        return false;
      const childRel = rel2 ? path4.join(rel2, e.name) : e.name;
      if (e.isDirectory()) {
        if (!await inner(childRel, depth + 1))
          return false;
      } else if (e.isFile()) {
        if (await fn(childRel) === false)
          return false;
      }
    }
    return true;
  }
  await inner(rel, 0);
}
async function findRg() {
  const dirs = (process.env["PATH"] ?? "").split(path4.delimiter);
  for (const d of dirs) {
    const candidate = path4.join(d, "rg");
    try {
      await fs3.access(candidate, fssync2.constants.X_OK);
      return candidate;
    } catch {
    }
  }
  return null;
}
var _BashSession_instances, _BashSession_proc, _BashSession_buf, _BashSession_truncated, _BashSession_closed, _BashSession_waiting, _BashSession_append, BASH_OUTPUT_LIMIT, BASH_DEFAULT_TIMEOUT_MS, DEFAULT_MAX_FILE_BYTES, GREP_OUTPUT_LIMIT, GREP_MAX_LINE_LENGTH, GLOB_RESULT_LIMIT, BashTimeoutError, ANSI_RE, fsGlob, BashSession, WALK_MAX_DEPTH, WALK_MAX_ENTRIES;
var init_node = __esm({
  "node_modules/@anthropic-ai/sdk/tools/agent-toolset/node.mjs"() {
    init_tslib();
    init_error();
    init_ToolError();
    init_json_schema();
    init_promise();
    init_fs_util();
    init_skills();
    BASH_OUTPUT_LIMIT = 100 * 1024;
    BASH_DEFAULT_TIMEOUT_MS = 12e4;
    DEFAULT_MAX_FILE_BYTES = 256 * 1024;
    GREP_OUTPUT_LIMIT = 100 * 1024;
    GREP_MAX_LINE_LENGTH = 2e3;
    GLOB_RESULT_LIMIT = 200;
    BashTimeoutError = class extends AnthropicError {
      constructor(timeoutMs) {
        super(`bash command timed out after ${timeoutMs}ms`);
        this.name = "BashTimeoutError";
        this.timeoutMs = timeoutMs;
      }
    };
    ANSI_RE = /\x1b\[[0-9;?]*[ -/]*[@-~]/g;
    fsGlob = fs3.glob;
    BashSession = class {
      constructor(dir, env = scrubbedShellEnv()) {
        _BashSession_instances.add(this);
        _BashSession_proc.set(this, void 0);
        _BashSession_buf.set(this, "");
        _BashSession_truncated.set(this, false);
        _BashSession_closed.set(this, false);
        _BashSession_waiting.set(this, null);
        __classPrivateFieldSet(this, _BashSession_proc, cp.spawn("/bin/bash", ["--noprofile", "--norc"], {
          cwd: dir,
          // `env` is the full base environment (the scrubbed process env by
          // default, or the verbatim replacement from `AgentToolContext.env`).
          // PS1/PS2/TERM are shell-control settings BashSession always applies so
          // the pipe-based sentinel exec parsing works — not part of the
          // user-facing environment.
          env: { ...env, PS1: "", PS2: "", TERM: "dumb" },
          stdio: ["pipe", "pipe", "pipe"],
          detached: true
        }), "f");
        __classPrivateFieldGet(this, _BashSession_proc, "f").stdout.setEncoding("utf8");
        __classPrivateFieldGet(this, _BashSession_proc, "f").stderr.setEncoding("utf8");
        __classPrivateFieldGet(this, _BashSession_proc, "f").stdout.on("data", (d) => __classPrivateFieldGet(this, _BashSession_instances, "m", _BashSession_append).call(this, d));
        __classPrivateFieldGet(this, _BashSession_proc, "f").stderr.on("data", (d) => __classPrivateFieldGet(this, _BashSession_instances, "m", _BashSession_append).call(this, d));
        __classPrivateFieldGet(this, _BashSession_proc, "f").once("close", () => {
          __classPrivateFieldSet(this, _BashSession_closed, true, "f");
          const w = __classPrivateFieldGet(this, _BashSession_waiting, "f");
          __classPrivateFieldSet(this, _BashSession_waiting, null, "f");
          w?.resolve();
        });
      }
      /** Whether the underlying shell process has exited. */
      get closed() {
        return __classPrivateFieldGet(this, _BashSession_closed, "f");
      }
      async exec(command, opts = {}) {
        if (__classPrivateFieldGet(this, _BashSession_closed, "f")) {
          throw new AnthropicError("bash session terminated");
        }
        const timeoutMs = opts.timeoutMs ?? BASH_DEFAULT_TIMEOUT_MS;
        const signal = opts.signal;
        signal?.throwIfAborted();
        __classPrivateFieldSet(this, _BashSession_buf, "", "f");
        __classPrivateFieldSet(this, _BashSession_truncated, false, "f");
        const sentinel2 = `__ANT_CMD_${crypto.randomUUID()}_DONE__`;
        const sentinelSplit = `${sentinel2.slice(0, 8)}''${sentinel2.slice(8)}`;
        const wrapped = `{ ${command}
} </dev/null 2>&1; printf '\\n${sentinelSplit}%d\\n' $?
`;
        __classPrivateFieldGet(this, _BashSession_proc, "f").stdin.write(wrapped);
        if (__classPrivateFieldGet(this, _BashSession_buf, "f").indexOf(sentinel2) < 0) {
          const { promise: sentinelSeen, resolve: resolve4 } = promiseWithResolvers();
          __classPrivateFieldSet(this, _BashSession_waiting, { sentinel: sentinel2, resolve: resolve4 }, "f");
          let timer;
          let onAbort;
          try {
            await Promise.race([
              sentinelSeen,
              new Promise((_, reject) => {
                timer = setTimeout(() => reject(new BashTimeoutError(timeoutMs)), timeoutMs);
              }),
              new Promise((_, reject) => {
                if (!signal)
                  return;
                onAbort = () => reject(signal.reason);
                signal.addEventListener("abort", onAbort, { once: true });
              })
            ]);
          } finally {
            if (timer)
              clearTimeout(timer);
            if (onAbort && signal)
              signal.removeEventListener("abort", onAbort);
            __classPrivateFieldSet(this, _BashSession_waiting, null, "f");
          }
        }
        const idx = __classPrivateFieldGet(this, _BashSession_buf, "f").indexOf(sentinel2);
        if (idx < 0) {
          throw new AnthropicError("bash session terminated");
        }
        const tail = __classPrivateFieldGet(this, _BashSession_buf, "f").slice(idx + sentinel2.length);
        const m = tail.match(/^(-?\d+)/);
        const exitCode = m ? parseInt(m[1], 10) : -1;
        let out = __classPrivateFieldGet(this, _BashSession_buf, "f").slice(0, idx).replace(ANSI_RE, "").replace(/\n+$/, "");
        if (__classPrivateFieldGet(this, _BashSession_truncated, "f")) {
          out = `[output truncated]
${out}`;
        }
        return { output: out, exitCode };
      }
      close() {
        if (__classPrivateFieldGet(this, _BashSession_closed, "f"))
          return;
        __classPrivateFieldSet(this, _BashSession_closed, true, "f");
        const w = __classPrivateFieldGet(this, _BashSession_waiting, "f");
        __classPrivateFieldSet(this, _BashSession_waiting, null, "f");
        w?.resolve();
        __classPrivateFieldGet(this, _BashSession_proc, "f").stdout.destroy();
        __classPrivateFieldGet(this, _BashSession_proc, "f").stderr.destroy();
        __classPrivateFieldGet(this, _BashSession_proc, "f").stdin.destroy();
        try {
          process.kill(-__classPrivateFieldGet(this, _BashSession_proc, "f").pid, "SIGKILL");
        } catch {
          __classPrivateFieldGet(this, _BashSession_proc, "f").kill("SIGKILL");
        }
        __classPrivateFieldGet(this, _BashSession_proc, "f").unref();
      }
    };
    _BashSession_proc = /* @__PURE__ */ new WeakMap(), _BashSession_buf = /* @__PURE__ */ new WeakMap(), _BashSession_truncated = /* @__PURE__ */ new WeakMap(), _BashSession_closed = /* @__PURE__ */ new WeakMap(), _BashSession_waiting = /* @__PURE__ */ new WeakMap(), _BashSession_instances = /* @__PURE__ */ new WeakSet(), _BashSession_append = function _BashSession_append2(d) {
      __classPrivateFieldSet(this, _BashSession_buf, __classPrivateFieldGet(this, _BashSession_buf, "f") + d, "f");
      if (__classPrivateFieldGet(this, _BashSession_buf, "f").length > BASH_OUTPUT_LIMIT) {
        __classPrivateFieldSet(this, _BashSession_buf, __classPrivateFieldGet(this, _BashSession_buf, "f").slice(__classPrivateFieldGet(this, _BashSession_buf, "f").length - BASH_OUTPUT_LIMIT), "f");
        __classPrivateFieldSet(this, _BashSession_truncated, true, "f");
      }
      if (__classPrivateFieldGet(this, _BashSession_waiting, "f") && __classPrivateFieldGet(this, _BashSession_buf, "f").indexOf(__classPrivateFieldGet(this, _BashSession_waiting, "f").sentinel) >= 0) {
        const w = __classPrivateFieldGet(this, _BashSession_waiting, "f");
        __classPrivateFieldSet(this, _BashSession_waiting, null, "f");
        w.resolve();
      }
    };
    WALK_MAX_DEPTH = 40;
    WALK_MAX_ENTRIES = 5e4;
  }
});

// node_modules/@anthropic-ai/sdk/lib/environments/worker.mjs
async function forceStop(client, work, log, requestOptions) {
  try {
    await client.beta.environments.work.stop(
      work.id,
      { environment_id: work.environment_id, force: true },
      // Caller's headers pass through; the helper-tag header is on the scoped
      // sub-client's default_headers via copyClientForHelper, so no per-call
      // re-stamping needed.
      { ...requestOptions, headers: buildHeaders([requestOptions?.headers]) }
    );
  } catch (e) {
    if (!isStatus(e, 409)) {
      log.error("force-stop on exit failed", { work_id: work.id, error: String(e) });
    }
  }
}
async function heartbeatLoop(client, work, ctrl, logger, requestOptions) {
  let intervalMs = HEARTBEAT_DEFAULT_MS;
  let last = NO_HEARTBEAT_SENTINEL;
  const beat = async () => {
    try {
      const resp = await client.beta.environments.work.heartbeat(work.id, { environment_id: work.environment_id, expected_last_heartbeat: last }, { ...requestOptions, headers: buildHeaders([requestOptions?.headers]), signal: ctrl.signal });
      last = resp.last_heartbeat;
      if (resp.ttl_seconds > 0) {
        intervalMs = Math.max(1e3, Math.min(resp.ttl_seconds * 1e3 / 2, HEARTBEAT_DEFAULT_MS));
      }
      if (resp.state === "stopping" || resp.state === "stopped") {
        logger.info("heartbeat signals shutdown", { work_id: work.id, state: resp.state });
        ctrl.abort();
      }
      if (!resp.lease_extended) {
        logger.warn("lease not extended, shutting down", { work_id: work.id });
        ctrl.abort();
      }
    } catch (e) {
      ctrl.signal.throwIfAborted();
      if (isFatal4xx(e)) {
        logger.error("permanent heartbeat failure", { work_id: work.id, error: String(e) });
        ctrl.abort();
        throw e;
      }
      logger.warn("transient heartbeat failure", { work_id: work.id, error: String(e) });
    }
  };
  await beat();
  while (!ctrl.signal.aborted) {
    await sleep(intervalMs, ctrl.signal);
    ctrl.signal.throwIfAborted();
    await beat();
  }
}
var _EnvironmentWorker_instances, _EnvironmentWorker_signal, _EnvironmentWorker_handleItem, HEARTBEAT_DEFAULT_MS, NO_HEARTBEAT_SENTINEL, EnvironmentWorker;
var init_worker = __esm({
  "node_modules/@anthropic-ai/sdk/lib/environments/worker.mjs"() {
    init_tslib();
    init_error();
    init_log();
    init_env();
    init_sleep();
    init_backoff();
    init_abort();
    init_headers();
    init_SessionToolRunner();
    init_poller();
    init_helper_client();
    HEARTBEAT_DEFAULT_MS = 3e4;
    NO_HEARTBEAT_SENTINEL = "NO_HEARTBEAT";
    EnvironmentWorker = class {
      constructor(opts) {
        _EnvironmentWorker_instances.add(this);
        _EnvironmentWorker_signal.set(this, void 0);
        this.client = opts.client;
        this.environmentId = opts.environmentId;
        this.environmentKey = opts.environmentKey;
        this.tools = opts.tools;
        this.workdir = opts.workdir ?? process.cwd();
        this.unrestrictedPaths = opts.unrestrictedPaths;
        this.maxFileBytes = opts.maxFileBytes;
        this.maxIdleMs = opts.maxIdleMs;
        this.workerId = opts.workerId;
        this.requestOptions = opts.requestOptions;
        __classPrivateFieldSet(this, _EnvironmentWorker_signal, opts.signal, "f");
      }
      /**
       * Poll the environment and service each claimed session until the supplied
       * signal (or the one passed to the constructor) aborts. Throws if
       * `environmentId` / `environmentKey` were not provided to the constructor.
       */
      async run(signal) {
        const { environmentId, environmentKey } = this;
        if (environmentId === void 0 || environmentKey === void 0) {
          throw new AnthropicError("EnvironmentWorker.run: environmentId and environmentKey are required to poll for work");
        }
        const externalSignal = signal ?? __classPrivateFieldGet(this, _EnvironmentWorker_signal, "f");
        const poller = new WorkPoller({
          client: this.client,
          environmentId,
          environmentKey,
          ...this.workerId !== void 0 ? { workerId: this.workerId } : {},
          ...externalSignal ? { signal: externalSignal } : {},
          ...this.requestOptions !== void 0 ? { requestOptions: this.requestOptions } : {},
          // The per-item handler force-stops every work item on exit; let it be the
          // single owner of `work.stop` rather than double-posting from the poller.
          autoStop: false
        });
        for await (const work of poller) {
          await __classPrivateFieldGet(this, _EnvironmentWorker_instances, "m", _EnvironmentWorker_handleItem).call(this, work, environmentKey, poller.signal);
        }
      }
      /**
       * Service a single, already-claimed work item without the poll loop: build the
       * per-session {@link AgentToolContext} (workdir from this worker's options),
       * download the session agent's skills (`setupSkills`), run a
       * {@link SessionToolRunner} for the session while heartbeating the work-item
       * lease in parallel, and force-stop the work item on exit (whether the runner
       * finishes normally, throws, or the heartbeat loop signals shutdown).
       *
       * Use this when something else does the claiming — e.g. a `worker poll
       * --on-work` script that hands an already-claimed item to a fresh process. The
       * work id / environment id / session id each fall back to `ANTHROPIC_WORK_ID` /
       * `ANTHROPIC_ENVIRONMENT_ID` / `ANTHROPIC_SESSION_ID` (the env vars that
       * command sets) when not passed; the environment key resolves from this
       * option, then the worker's own `environmentKey`, then
       * `ANTHROPIC_ENVIRONMENT_KEY`. With no arguments inside that command it just
       * works. Throws a clear error naming the first of the four required values
       * still missing after resolution.
       */
      async handleItem(opts) {
        const workId = opts?.workId ?? readEnv("ANTHROPIC_WORK_ID");
        const environmentId = opts?.environmentId ?? readEnv("ANTHROPIC_ENVIRONMENT_ID");
        const sessionId = opts?.sessionId ?? readEnv("ANTHROPIC_SESSION_ID");
        const environmentKey = opts?.environmentKey ?? this.environmentKey ?? readEnv("ANTHROPIC_ENVIRONMENT_KEY");
        if (!workId) {
          throw new AnthropicError("handleItem: workId is required \u2014 pass it or set ANTHROPIC_WORK_ID");
        }
        if (!environmentId) {
          throw new AnthropicError("handleItem: environmentId is required \u2014 pass it or set ANTHROPIC_ENVIRONMENT_ID");
        }
        if (!sessionId) {
          throw new AnthropicError("handleItem: sessionId is required \u2014 pass it or set ANTHROPIC_SESSION_ID");
        }
        if (!environmentKey) {
          throw new AnthropicError("handleItem: environmentKey is required \u2014 pass it, construct the worker with it, or set ANTHROPIC_ENVIRONMENT_KEY");
        }
        const work = {
          id: workId,
          environment_id: environmentId,
          data: { type: "session", id: sessionId }
        };
        await __classPrivateFieldGet(this, _EnvironmentWorker_instances, "m", _EnvironmentWorker_handleItem).call(this, work, environmentKey, opts?.signal ?? __classPrivateFieldGet(this, _EnvironmentWorker_signal, "f"));
      }
    };
    _EnvironmentWorker_signal = /* @__PURE__ */ new WeakMap(), _EnvironmentWorker_instances = /* @__PURE__ */ new WeakSet(), _EnvironmentWorker_handleItem = /**
     * The per-item body shared by {@link EnvironmentWorker.run}'s poll loop and
     * {@link EnvironmentWorker.handleItem}: run a {@link SessionToolRunner} for the
     * work item's session while heartbeating its lease, force-stopping on exit.
     * Non-session work items are ignored.
     */
    async function _EnvironmentWorker_handleItem2(work, environmentKey, externalSignal) {
      const log = loggerFor(this.client);
      const sessionClient = copyClientForHelper(this.client, {
        authToken: environmentKey,
        helper: "environments-worker"
      });
      const sessionId = work.data.id;
      const ctx = {
        workdir: this.workdir,
        client: this.client,
        sessionId,
        ...this.unrestrictedPaths !== void 0 ? { unrestrictedPaths: this.unrestrictedPaths } : {},
        ...this.maxFileBytes !== void 0 ? { maxFileBytes: this.maxFileBytes } : {}
      };
      const agentToolset = await Promise.resolve().then(() => (init_node(), node_exports));
      let cleanupSkills = async () => {
      };
      try {
        cleanupSkills = await agentToolset.setupSkills(ctx);
      } catch (e) {
        log.warn("skill setup failed", { session_id: sessionId, work_id: work.id, error: String(e) });
      }
      const tools = typeof this.tools === "function" ? this.tools(ctx) : this.tools ?? agentToolset.betaAgentToolset20260401(ctx);
      const ctrl = new AbortController();
      const detachExternal = linkAbort(externalSignal, ctrl);
      const heartbeatPromise = heartbeatLoop(sessionClient, work, ctrl, log, this.requestOptions).catch((e) => {
        if (!ctrl.signal.aborted)
          log.error("heartbeat loop failed", { work_id: work.id, error: String(e) });
        ctrl.abort();
      });
      try {
        const runner = new SessionToolRunner(sessionId, {
          client: sessionClient,
          tools,
          ...this.maxIdleMs !== void 0 ? { maxIdleMs: this.maxIdleMs } : {},
          ...this.requestOptions !== void 0 ? { requestOptions: this.requestOptions } : {},
          signal: ctrl.signal
        });
        for await (const _ of runner) {
        }
      } finally {
        ctrl.abort();
        detachExternal();
        await heartbeatPromise;
        await cleanupSkills().catch((e) => {
          log.warn("skill cleanup failed", { session_id: sessionId, work_id: work.id, error: String(e) });
        });
        await forceStop(sessionClient, work, log, this.requestOptions);
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/environments/work.mjs
var Work;
var init_work = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/environments/work.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    init_poller();
    init_worker();
    init_poller();
    init_worker();
    Work = class extends APIResource {
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * Retrieve detailed information about a specific work item.
       *
       * @example
       * ```ts
       * const betaSelfHostedWork =
       *   await client.beta.environments.work.retrieve('work_id', {
       *     environment_id: 'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   });
       * ```
       */
      retrieve(workID, params, options) {
        const { environment_id, betas } = params;
        return this._client.get(path`/v1/environments/${environment_id}/work/${workID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * Update work item metadata with merge semantics.
       *
       * @example
       * ```ts
       * const betaSelfHostedWork =
       *   await client.beta.environments.work.update('work_id', {
       *     environment_id: 'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *     metadata: { foo: 'string' },
       *   });
       * ```
       */
      update(workID, params, options) {
        const { environment_id, betas, ...body } = params;
        return this._client.post(path`/v1/environments/${environment_id}/work/${workID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * List work items in an environment.
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaSelfHostedWork of client.beta.environments.work.list(
       *   'env_011CZkZ9X2dpNyB7HsEFoRfW',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(environmentID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/environments/${environmentID}/work?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * Acknowledge receipt of a work item, transitioning it from 'queued' to 'starting'
       * and removing it from the queue.
       *
       * @example
       * ```ts
       * const betaSelfHostedWork =
       *   await client.beta.environments.work.ack('work_id', {
       *     environment_id: 'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   });
       * ```
       */
      ack(workID, params, options) {
        const { environment_id, betas } = params;
        return this._client.post(path`/v1/environments/${environment_id}/work/${workID}/ack?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * Record a heartbeat for a work item to maintain the lease.
       *
       * @example
       * ```ts
       * const betaSelfHostedWorkHeartbeatResponse =
       *   await client.beta.environments.work.heartbeat('work_id', {
       *     environment_id: 'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   });
       * ```
       */
      heartbeat(workID, params, options) {
        const { environment_id, desired_ttl_seconds, expected_last_heartbeat, betas } = params;
        return this._client.post(path`/v1/environments/${environment_id}/work/${workID}/heartbeat?beta=true`, {
          query: { desired_ttl_seconds, expected_last_heartbeat },
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * Long poll for work items in the queue.
       *
       * @example
       * ```ts
       * const betaSelfHostedWork =
       *   await client.beta.environments.work.poll(
       *     'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   );
       * ```
       */
      poll(environmentID, params = {}, options) {
        const { betas, "Anthropic-Worker-ID": anthropicWorkerID, ...query } = params ?? {};
        return this._client.get(path`/v1/environments/${environmentID}/work/poll?beta=true`, {
          query,
          ...options,
          headers: buildHeaders([
            {
              "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString(),
              ...anthropicWorkerID != null ? { "Anthropic-Worker-ID": anthropicWorkerID } : void 0
            },
            options?.headers
          ])
        });
      }
      /**
       * Get statistics about the work queue for an environment.
       *
       * @example
       * ```ts
       * const betaSelfHostedWorkQueueStats =
       *   await client.beta.environments.work.stats(
       *     'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   );
       * ```
       */
      stats(environmentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/environments/${environmentID}/work/stats?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Note: these endpoints are called automatically by the pre-built environment
       * worker provided in the SDKs and CLI, for orchestrating sessions with self-hosted
       * sandbox environments. They are included here as a reference; you do not need to
       * invoke them directly.
       *
       * Stop a work item, initiating graceful or forced shutdown.
       *
       * @example
       * ```ts
       * const betaSelfHostedWork =
       *   await client.beta.environments.work.stop('work_id', {
       *     environment_id: 'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   });
       * ```
       */
      stop(workID, params, options) {
        const { environment_id, betas, ...body } = params;
        return this._client.post(path`/v1/environments/${environment_id}/work/${workID}/stop?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Continuously claim work from a self-hosted environment, ack each item,
       * and yield it. Posts `stop` automatically when the consumer's loop body
       * returns or when iteration ends.
       *
       * @example
       * ```ts
       * for await (const work of client.beta.environments.work.poller({
       *   environmentId,
       *   environmentKey,
       * })) {
       *   if (work.data.type !== 'session') continue;
       *   // ...service the work...
       * }
       * ```
       */
      poller(opts) {
        return new WorkPoller({ ...opts, client: this._client });
      }
      /**
       * The self-hosted environment runner: poll for work, and for each claimed
       * session set up the workdir, download the agent's skills, run the tools while
       * heartbeating the lease, and force-stop on exit.
       *
       * @example
       * ```ts
       * // Long-running daemon — poll, serve each session, loop:
       * await client.beta.environments.work
       *   .worker({ environmentId, environmentKey, workdir: '/workspace' })
       *   .run();
       *
       * // Or service one already-claimed work item (e.g. inside a sandbox spawned
       * // by `ant worker poll --on-work`) — handleItem() reads the ANTHROPIC_* env vars:
       * await client.beta.environments.work.worker({ workdir: '/workspace' }).handleItem();
       * ```
       */
      worker(opts) {
        return new EnvironmentWorker({ ...opts, client: this._client });
      }
    };
    Work.WorkPoller = WorkPoller;
    Work.EnvironmentWorker = EnvironmentWorker;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/environments/environments.mjs
var Environments;
var init_environments = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/environments/environments.mjs"() {
    init_resource();
    init_work();
    init_work();
    init_pagination();
    init_headers();
    init_path();
    Environments = class extends APIResource {
      constructor() {
        super(...arguments);
        this.work = new Work(this._client);
      }
      /**
       * Create a new environment with the specified configuration.
       *
       * @example
       * ```ts
       * const betaEnvironment =
       *   await client.beta.environments.create({
       *     name: 'python-data-analysis',
       *   });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/environments?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Retrieve a specific environment by ID.
       *
       * @example
       * ```ts
       * const betaEnvironment =
       *   await client.beta.environments.retrieve(
       *     'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   );
       * ```
       */
      retrieve(environmentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/environments/${environmentID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update an existing environment's configuration.
       *
       * @example
       * ```ts
       * const betaEnvironment =
       *   await client.beta.environments.update(
       *     'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   );
       * ```
       */
      update(environmentID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/environments/${environmentID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List environments with pagination support.
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaEnvironment of client.beta.environments.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/environments?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete an environment by ID. Returns a confirmation of the deletion.
       *
       * @example
       * ```ts
       * const betaEnvironmentDeleteResponse =
       *   await client.beta.environments.delete(
       *     'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   );
       * ```
       */
      delete(environmentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/environments/${environmentID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive an environment by ID. Archived environments cannot be used to create new
       * sessions.
       *
       * @example
       * ```ts
       * const betaEnvironment =
       *   await client.beta.environments.archive(
       *     'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   );
       * ```
       */
      archive(environmentID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/environments/${environmentID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
    Environments.Work = Work;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/memory-stores/memories.mjs
var Memories;
var init_memories = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/memory-stores/memories.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Memories = class extends APIResource {
      /**
       * Create a memory
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemory =
       *   await client.beta.memoryStores.memories.create(
       *     'memory_store_id',
       *     { content: 'content', path: 'xx' },
       *   );
       * ```
       */
      create(memoryStoreID, params, options) {
        const { view, betas, ...body } = params;
        return this._client.post(path`/v1/memory_stores/${memoryStoreID}/memories?beta=true`, {
          query: { view },
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Retrieve a memory
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemory =
       *   await client.beta.memoryStores.memories.retrieve(
       *     'memory_id',
       *     { memory_store_id: 'memory_store_id' },
       *   );
       * ```
       */
      retrieve(memoryID, params, options) {
        const { memory_store_id, betas, ...query } = params;
        return this._client.get(path`/v1/memory_stores/${memory_store_id}/memories/${memoryID}?beta=true`, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update a memory
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemory =
       *   await client.beta.memoryStores.memories.update(
       *     'memory_id',
       *     { memory_store_id: 'memory_store_id' },
       *   );
       * ```
       */
      update(memoryID, params, options) {
        const { memory_store_id, view, betas, ...body } = params;
        return this._client.post(path`/v1/memory_stores/${memory_store_id}/memories/${memoryID}?beta=true`, {
          query: { view },
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List memories
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsMemoryListItem of client.beta.memoryStores.memories.list(
       *   'memory_store_id',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(memoryStoreID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/memory_stores/${memoryStoreID}/memories?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete a memory
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeletedMemory =
       *   await client.beta.memoryStores.memories.delete(
       *     'memory_id',
       *     { memory_store_id: 'memory_store_id' },
       *   );
       * ```
       */
      delete(memoryID, params, options) {
        const { memory_store_id, expected_content_sha256, betas } = params;
        return this._client.delete(path`/v1/memory_stores/${memory_store_id}/memories/${memoryID}?beta=true`, {
          query: { expected_content_sha256 },
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/memory-stores/memory-versions.mjs
var MemoryVersions;
var init_memory_versions = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/memory-stores/memory-versions.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    MemoryVersions = class extends APIResource {
      /**
       * Retrieve a memory version
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemoryVersion =
       *   await client.beta.memoryStores.memoryVersions.retrieve(
       *     'memory_version_id',
       *     { memory_store_id: 'memory_store_id' },
       *   );
       * ```
       */
      retrieve(memoryVersionID, params, options) {
        const { memory_store_id, betas, ...query } = params;
        return this._client.get(path`/v1/memory_stores/${memory_store_id}/memory_versions/${memoryVersionID}?beta=true`, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List memory versions
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsMemoryVersion of client.beta.memoryStores.memoryVersions.list(
       *   'memory_store_id',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(memoryStoreID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/memory_stores/${memoryStoreID}/memory_versions?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Redact a memory version
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemoryVersion =
       *   await client.beta.memoryStores.memoryVersions.redact(
       *     'memory_version_id',
       *     { memory_store_id: 'memory_store_id' },
       *   );
       * ```
       */
      redact(memoryVersionID, params, options) {
        const { memory_store_id, betas } = params;
        return this._client.post(path`/v1/memory_stores/${memory_store_id}/memory_versions/${memoryVersionID}/redact?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/memory-stores/memory-stores.mjs
var MemoryStores;
var init_memory_stores = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/memory-stores/memory-stores.mjs"() {
    init_resource();
    init_memories();
    init_memories();
    init_memory_versions();
    init_memory_versions();
    init_pagination();
    init_headers();
    init_path();
    MemoryStores = class extends APIResource {
      constructor() {
        super(...arguments);
        this.memories = new Memories(this._client);
        this.memoryVersions = new MemoryVersions(this._client);
      }
      /**
       * Create a memory store
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemoryStore =
       *   await client.beta.memoryStores.create({ name: 'x' });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/memory_stores?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Retrieve a memory store
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemoryStore =
       *   await client.beta.memoryStores.retrieve(
       *     'memory_store_id',
       *   );
       * ```
       */
      retrieve(memoryStoreID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/memory_stores/${memoryStoreID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update a memory store
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemoryStore =
       *   await client.beta.memoryStores.update('memory_store_id');
       * ```
       */
      update(memoryStoreID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/memory_stores/${memoryStoreID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List memory stores
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsMemoryStore of client.beta.memoryStores.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/memory_stores?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete a memory store
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeletedMemoryStore =
       *   await client.beta.memoryStores.delete('memory_store_id');
       * ```
       */
      delete(memoryStoreID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/memory_stores/${memoryStoreID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive a memory store
       *
       * @example
       * ```ts
       * const betaManagedAgentsMemoryStore =
       *   await client.beta.memoryStores.archive('memory_store_id');
       * ```
       */
      archive(memoryStoreID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/memory_stores/${memoryStoreID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "agent-memory-2026-07-22"].toString() },
            options?.headers
          ])
        });
      }
    };
    MemoryStores.Memories = Memories;
    MemoryStores.MemoryVersions = MemoryVersions;
  }
});

// node_modules/@anthropic-ai/sdk/error.mjs
var init_error2 = __esm({
  "node_modules/@anthropic-ai/sdk/error.mjs"() {
    init_error();
  }
});

// node_modules/@anthropic-ai/sdk/internal/decoders/jsonl.mjs
var JSONLDecoder;
var init_jsonl = __esm({
  "node_modules/@anthropic-ai/sdk/internal/decoders/jsonl.mjs"() {
    init_error();
    init_shims();
    init_line();
    JSONLDecoder = class _JSONLDecoder {
      constructor(iterator, controller) {
        this.iterator = iterator;
        this.controller = controller;
      }
      async *decoder() {
        const lineDecoder = new LineDecoder();
        for await (const chunk of this.iterator) {
          for (const line of lineDecoder.decode(chunk)) {
            yield JSON.parse(line);
          }
        }
        for (const line of lineDecoder.flush()) {
          yield JSON.parse(line);
        }
      }
      [Symbol.asyncIterator]() {
        return this.decoder();
      }
      static fromResponse(response, controller) {
        if (!response.body) {
          controller.abort();
          if (typeof globalThis.navigator !== "undefined" && globalThis.navigator.product === "ReactNative") {
            throw new AnthropicError(`The default react-native fetch implementation does not support streaming. Please use expo/fetch: https://docs.expo.dev/versions/latest/sdk/expo/#expofetch-api`);
          }
          throw new AnthropicError(`Attempted to iterate over a response with no body`);
        }
        return new _JSONLDecoder(ReadableStreamToAsyncIterable(response.body), controller);
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/messages/batches.mjs
var Batches;
var init_batches = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/messages/batches.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_jsonl();
    init_error2();
    init_path();
    Batches = class extends APIResource {
      /**
       * Send a batch of Message creation requests.
       *
       * The Message Batches API can be used to process multiple Messages API requests at
       * once. Once a Message Batch is created, it begins processing immediately. Batches
       * can take up to 24 hours to complete.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const betaMessageBatch =
       *   await client.beta.messages.batches.create({
       *     requests: [
       *       {
       *         custom_id: 'my-custom-id-1',
       *         params: {
       *           max_tokens: 1024,
       *           messages: [
       *             { content: 'Hello, world', role: 'user' },
       *           ],
       *           model: 'claude-opus-4-6',
       *         },
       *       },
       *     ],
       *   });
       * ```
       */
      create(params, options) {
        const { betas, user_profile_id, ...body } = params;
        return this._client.post("/v1/messages/batches?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            {
              "anthropic-beta": [...betas ?? [], "message-batches-2024-09-24"].toString(),
              ...user_profile_id != null ? { "anthropic-user-profile-id": user_profile_id } : void 0
            },
            options?.headers
          ])
        });
      }
      /**
       * This endpoint is idempotent and can be used to poll for Message Batch
       * completion. To access the results of a Message Batch, make a request to the
       * `results_url` field in the response.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const betaMessageBatch =
       *   await client.beta.messages.batches.retrieve(
       *     'message_batch_id',
       *   );
       * ```
       */
      retrieve(messageBatchID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/messages/batches/${messageBatchID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "message-batches-2024-09-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List all Message Batches within a Workspace. Most recently created batches are
       * returned first.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaMessageBatch of client.beta.messages.batches.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/messages/batches?beta=true", Page, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "message-batches-2024-09-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete a Message Batch.
       *
       * Message Batches can only be deleted once they've finished processing. If you'd
       * like to delete an in-progress batch, you must first cancel it.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const betaDeletedMessageBatch =
       *   await client.beta.messages.batches.delete(
       *     'message_batch_id',
       *   );
       * ```
       */
      delete(messageBatchID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/messages/batches/${messageBatchID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "message-batches-2024-09-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Batches may be canceled any time before processing ends. Once cancellation is
       * initiated, the batch enters a `canceling` state, at which time the system may
       * complete any in-progress, non-interruptible requests before finalizing
       * cancellation.
       *
       * The number of canceled requests is specified in `request_counts`. To determine
       * which requests were canceled, check the individual results within the batch.
       * Note that cancellation may not result in any canceled requests if they were
       * non-interruptible.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const betaMessageBatch =
       *   await client.beta.messages.batches.cancel(
       *     'message_batch_id',
       *   );
       * ```
       */
      cancel(messageBatchID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/messages/batches/${messageBatchID}/cancel?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "message-batches-2024-09-24"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Streams the results of a Message Batch as a `.jsonl` file.
       *
       * Each line in the file is a JSON object containing the result of a single request
       * in the Message Batch. Results are not guaranteed to be in the same order as
       * requests. Use the `custom_id` field to match results to requests.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const betaMessageBatchIndividualResponse =
       *   await client.beta.messages.batches.results(
       *     'message_batch_id',
       *   );
       * ```
       */
      async results(messageBatchID, params = {}, options) {
        const batch = await this.retrieve(messageBatchID);
        if (!batch.results_url) {
          throw new AnthropicError(`No batch \`results_url\`; Has it finished processing? ${batch.processing_status} - ${batch.id}`);
        }
        const { betas } = params ?? {};
        return this._client.get(batch.results_url, {
          ...options,
          headers: buildHeaders([
            {
              "anthropic-beta": [...betas ?? [], "message-batches-2024-09-24"].toString(),
              Accept: "application/binary"
            },
            options?.headers
          ]),
          stream: true,
          __binaryResponse: true
        })._thenUnwrap((_, props) => JSONLDecoder.fromResponse(props.response, props.controller));
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/internal/constants.mjs
var MODEL_NONSTREAMING_TOKENS;
var init_constants = __esm({
  "node_modules/@anthropic-ai/sdk/internal/constants.mjs"() {
    MODEL_NONSTREAMING_TOKENS = {
      "claude-opus-4@20250514": 8192,
      "anthropic.claude-opus-4-1-20250805-v1:0": 8192,
      "claude-opus-4-1@20250805": 8192
    };
  }
});

// node_modules/@anthropic-ai/sdk/lib/beta-parser.mjs
function getOutputFormat(params) {
  return params?.output_format ?? params?.output_config?.format;
}
function maybeParseBetaMessage(message, params, opts) {
  const outputFormat = getOutputFormat(params);
  if (!params || !("parse" in (outputFormat ?? {}))) {
    return {
      ...message,
      content: message.content.map((block) => {
        if (block.type === "text") {
          const parsedBlock = Object.defineProperty({ ...block }, "parsed_output", {
            value: null,
            enumerable: false
          });
          return Object.defineProperty(parsedBlock, "parsed", {
            get() {
              opts.logger.warn("The `parsed` property on `text` blocks is deprecated, please use `parsed_output` instead.");
              return null;
            },
            enumerable: false
          });
        }
        return block;
      }),
      parsed_output: null
    };
  }
  return parseBetaMessage(message, params, opts);
}
function parseBetaMessage(message, params, opts) {
  let firstParsedOutput = null;
  const content = message.content.map((block) => {
    if (block.type === "text") {
      const parsedOutput = parseBetaOutputFormat(params, block.text);
      if (firstParsedOutput === null) {
        firstParsedOutput = parsedOutput;
      }
      const parsedBlock = Object.defineProperty({ ...block }, "parsed_output", {
        value: parsedOutput,
        enumerable: false
      });
      return Object.defineProperty(parsedBlock, "parsed", {
        get() {
          opts.logger.warn("The `parsed` property on `text` blocks is deprecated, please use `parsed_output` instead.");
          return parsedOutput;
        },
        enumerable: false
      });
    }
    return block;
  });
  return {
    ...message,
    content,
    parsed_output: firstParsedOutput
  };
}
function parseBetaOutputFormat(params, content) {
  const outputFormat = getOutputFormat(params);
  if (outputFormat?.type !== "json_schema") {
    return null;
  }
  try {
    if ("parse" in outputFormat) {
      return outputFormat.parse(content);
    }
    return JSON.parse(content);
  } catch (error) {
    throw new AnthropicError(`Failed to parse structured output: ${error}`);
  }
}
var init_beta_parser = __esm({
  "node_modules/@anthropic-ai/sdk/lib/beta-parser.mjs"() {
    init_error();
  }
});

// node_modules/@anthropic-ai/sdk/streaming.mjs
var init_streaming2 = __esm({
  "node_modules/@anthropic-ai/sdk/streaming.mjs"() {
    init_streaming();
  }
});

// node_modules/@anthropic-ai/sdk/_vendor/partial-json-parser/parser.mjs
var tokenize, strip, unstrip, generate, partialParse;
var init_parser = __esm({
  "node_modules/@anthropic-ai/sdk/_vendor/partial-json-parser/parser.mjs"() {
    tokenize = (input) => {
      let current = 0;
      let tokens = [];
      while (current < input.length) {
        let char = input[current];
        if (char === "\\") {
          current++;
          continue;
        }
        if (char === "{") {
          tokens.push({
            type: "brace",
            value: "{"
          });
          current++;
          continue;
        }
        if (char === "}") {
          tokens.push({
            type: "brace",
            value: "}"
          });
          current++;
          continue;
        }
        if (char === "[") {
          tokens.push({
            type: "paren",
            value: "["
          });
          current++;
          continue;
        }
        if (char === "]") {
          tokens.push({
            type: "paren",
            value: "]"
          });
          current++;
          continue;
        }
        if (char === ":") {
          tokens.push({
            type: "separator",
            value: ":"
          });
          current++;
          continue;
        }
        if (char === ",") {
          tokens.push({
            type: "delimiter",
            value: ","
          });
          current++;
          continue;
        }
        if (char === '"') {
          let value = "";
          let danglingQuote = false;
          char = input[++current];
          while (char !== '"') {
            if (current === input.length) {
              danglingQuote = true;
              break;
            }
            if (char === "\\") {
              current++;
              if (current === input.length) {
                danglingQuote = true;
                break;
              }
              value += char + input[current];
              char = input[++current];
            } else {
              value += char;
              char = input[++current];
            }
          }
          char = input[++current];
          if (!danglingQuote) {
            tokens.push({
              type: "string",
              value
            });
          }
          continue;
        }
        let WHITESPACE = /\s/;
        if (char && WHITESPACE.test(char)) {
          current++;
          continue;
        }
        let NUMBERS = /[0-9]/;
        if (char && NUMBERS.test(char) || char === "-" || char === ".") {
          let value = "";
          if (char === "-") {
            value += char;
            char = input[++current];
          }
          while (char && (NUMBERS.test(char) || char === "." || // exponent marker, e.g. `1e10` or `1.5E-9`
          char === "e" || char === "E" || // exponent sign, only valid immediately after the exponent marker
          (char === "-" || char === "+") && (value[value.length - 1] === "e" || value[value.length - 1] === "E"))) {
            value += char;
            char = input[++current];
          }
          tokens.push({
            type: "number",
            value
          });
          continue;
        }
        let LETTERS = /[a-z]/i;
        if (char && LETTERS.test(char)) {
          let value = "";
          while (char && LETTERS.test(char)) {
            if (current === input.length) {
              break;
            }
            value += char;
            char = input[++current];
          }
          if (value == "true" || value == "false" || value === "null") {
            tokens.push({
              type: "name",
              value
            });
          } else {
            current++;
            continue;
          }
          continue;
        }
        current++;
      }
      return tokens;
    };
    strip = (tokens) => {
      if (tokens.length === 0) {
        return tokens;
      }
      let lastToken = tokens[tokens.length - 1];
      switch (lastToken.type) {
        case "separator":
          tokens = tokens.slice(0, tokens.length - 1);
          return strip(tokens);
          break;
        case "number":
          let lastCharacterOfLastToken = lastToken.value[lastToken.value.length - 1];
          if (lastCharacterOfLastToken === "." || lastCharacterOfLastToken === "-" || lastCharacterOfLastToken === "+" || lastCharacterOfLastToken === "e" || lastCharacterOfLastToken === "E") {
            tokens = tokens.slice(0, tokens.length - 1);
            return strip(tokens);
          }
        case "string":
          let tokenBeforeTheLastToken = tokens[tokens.length - 2];
          if (tokenBeforeTheLastToken?.type === "delimiter") {
            tokens = tokens.slice(0, tokens.length - 1);
            return strip(tokens);
          } else if (tokenBeforeTheLastToken?.type === "brace" && tokenBeforeTheLastToken.value === "{") {
            tokens = tokens.slice(0, tokens.length - 1);
            return strip(tokens);
          }
          break;
        case "delimiter":
          tokens = tokens.slice(0, tokens.length - 1);
          return strip(tokens);
          break;
      }
      return tokens;
    };
    unstrip = (tokens) => {
      let tail = [];
      tokens.map((token2) => {
        if (token2.type === "brace") {
          if (token2.value === "{") {
            tail.push("}");
          } else {
            tail.splice(tail.lastIndexOf("}"), 1);
          }
        }
        if (token2.type === "paren") {
          if (token2.value === "[") {
            tail.push("]");
          } else {
            tail.splice(tail.lastIndexOf("]"), 1);
          }
        }
      });
      if (tail.length > 0) {
        tail.reverse().map((item) => {
          if (item === "}") {
            tokens.push({
              type: "brace",
              value: "}"
            });
          } else if (item === "]") {
            tokens.push({
              type: "paren",
              value: "]"
            });
          }
        });
      }
      return tokens;
    };
    generate = (tokens) => {
      let output = "";
      tokens.map((token2) => {
        switch (token2.type) {
          case "string":
            output += '"' + token2.value + '"';
            break;
          default:
            output += token2.value;
            break;
        }
      });
      return output;
    };
    partialParse = (input) => JSON.parse(generate(unstrip(strip(tokenize(input)))));
  }
});

// node_modules/@anthropic-ai/sdk/internal/message-stream-utils.mjs
function withLazyInput(prev, jsonBuf) {
  const next = {};
  for (const key of Object.keys(prev)) {
    if (key !== "input")
      next[key] = prev[key];
  }
  Object.defineProperty(next, JSON_BUF_PROPERTY, { value: jsonBuf, enumerable: false, writable: true });
  let input;
  let parsed = false;
  Object.defineProperty(next, "input", {
    enumerable: true,
    configurable: true,
    get() {
      if (!parsed) {
        input = jsonBuf ? partialParse(jsonBuf) : {};
        parsed = true;
      }
      return input;
    }
  });
  return next;
}
var JSON_BUF_PROPERTY;
var init_message_stream_utils = __esm({
  "node_modules/@anthropic-ai/sdk/internal/message-stream-utils.mjs"() {
    init_parser();
    JSON_BUF_PROPERTY = "__json_buf";
  }
});

// node_modules/@anthropic-ai/sdk/lib/BetaMessageStream.mjs
function tracksToolInput(content) {
  return content.type === "tool_use" || content.type === "server_tool_use" || content.type === "mcp_tool_use";
}
function checkNever(x) {
}
var _BetaMessageStream_instances, _BetaMessageStream_currentMessageSnapshot, _BetaMessageStream_params, _BetaMessageStream_connectedPromise, _BetaMessageStream_resolveConnectedPromise, _BetaMessageStream_rejectConnectedPromise, _BetaMessageStream_endPromise, _BetaMessageStream_resolveEndPromise, _BetaMessageStream_rejectEndPromise, _BetaMessageStream_listeners, _BetaMessageStream_ended, _BetaMessageStream_errored, _BetaMessageStream_aborted, _BetaMessageStream_catchingPromiseCreated, _BetaMessageStream_response, _BetaMessageStream_request_id, _BetaMessageStream_logger, _BetaMessageStream_getFinalMessage, _BetaMessageStream_getFinalText, _BetaMessageStream_handleError, _BetaMessageStream_beginRequest, _BetaMessageStream_addStreamEvent, _BetaMessageStream_endRequest, _BetaMessageStream_accumulateMessage, _BetaMessageStream_toolInputParseError, BetaMessageStream;
var init_BetaMessageStream = __esm({
  "node_modules/@anthropic-ai/sdk/lib/BetaMessageStream.mjs"() {
    init_tslib();
    init_stainless_helper_header();
    init_error2();
    init_errors();
    init_streaming2();
    init_beta_parser();
    init_message_stream_utils();
    BetaMessageStream = class _BetaMessageStream {
      constructor(params, opts) {
        _BetaMessageStream_instances.add(this);
        this.messages = [];
        this.receivedMessages = [];
        _BetaMessageStream_currentMessageSnapshot.set(this, void 0);
        _BetaMessageStream_params.set(this, null);
        this.controller = new AbortController();
        _BetaMessageStream_connectedPromise.set(this, void 0);
        _BetaMessageStream_resolveConnectedPromise.set(this, () => {
        });
        _BetaMessageStream_rejectConnectedPromise.set(this, () => {
        });
        _BetaMessageStream_endPromise.set(this, void 0);
        _BetaMessageStream_resolveEndPromise.set(this, () => {
        });
        _BetaMessageStream_rejectEndPromise.set(this, () => {
        });
        _BetaMessageStream_listeners.set(this, {});
        _BetaMessageStream_ended.set(this, false);
        _BetaMessageStream_errored.set(this, false);
        _BetaMessageStream_aborted.set(this, false);
        _BetaMessageStream_catchingPromiseCreated.set(this, false);
        _BetaMessageStream_response.set(this, void 0);
        _BetaMessageStream_request_id.set(this, void 0);
        _BetaMessageStream_logger.set(this, void 0);
        _BetaMessageStream_handleError.set(this, (error) => {
          __classPrivateFieldSet(this, _BetaMessageStream_errored, true, "f");
          if (isAbortError(error)) {
            error = new APIUserAbortError();
          }
          if (error instanceof APIUserAbortError) {
            __classPrivateFieldSet(this, _BetaMessageStream_aborted, true, "f");
            return this._emit("abort", error);
          }
          if (error instanceof AnthropicError) {
            return this._emit("error", error);
          }
          if (error instanceof Error) {
            const anthropicError = new AnthropicError(error.message);
            anthropicError.cause = error;
            return this._emit("error", anthropicError);
          }
          return this._emit("error", new AnthropicError(String(error)));
        });
        __classPrivateFieldSet(this, _BetaMessageStream_connectedPromise, new Promise((resolve4, reject) => {
          __classPrivateFieldSet(this, _BetaMessageStream_resolveConnectedPromise, resolve4, "f");
          __classPrivateFieldSet(this, _BetaMessageStream_rejectConnectedPromise, reject, "f");
        }), "f");
        __classPrivateFieldSet(this, _BetaMessageStream_endPromise, new Promise((resolve4, reject) => {
          __classPrivateFieldSet(this, _BetaMessageStream_resolveEndPromise, resolve4, "f");
          __classPrivateFieldSet(this, _BetaMessageStream_rejectEndPromise, reject, "f");
        }), "f");
        __classPrivateFieldGet(this, _BetaMessageStream_connectedPromise, "f").catch(() => {
        });
        __classPrivateFieldGet(this, _BetaMessageStream_endPromise, "f").catch(() => {
        });
        __classPrivateFieldSet(this, _BetaMessageStream_params, params, "f");
        __classPrivateFieldSet(this, _BetaMessageStream_logger, opts?.logger ?? console, "f");
      }
      get response() {
        return __classPrivateFieldGet(this, _BetaMessageStream_response, "f");
      }
      get request_id() {
        return __classPrivateFieldGet(this, _BetaMessageStream_request_id, "f");
      }
      /**
       * Returns the `MessageStream` data, the raw `Response` instance and the ID of the request,
       * returned vie the `request-id` header which is useful for debugging requests and resporting
       * issues to Anthropic.
       *
       * This is the same as the `APIPromise.withResponse()` method.
       *
       * This method will raise an error if you created the stream using `MessageStream.fromReadableStream`
       * as no `Response` is available.
       */
      async withResponse() {
        __classPrivateFieldSet(this, _BetaMessageStream_catchingPromiseCreated, true, "f");
        const response = await __classPrivateFieldGet(this, _BetaMessageStream_connectedPromise, "f");
        if (!response) {
          throw new Error("Could not resolve a `Response` object");
        }
        return {
          data: this,
          response,
          request_id: response.headers.get("request-id")
        };
      }
      /**
       * Intended for use on the frontend, consuming a stream produced with
       * `.toReadableStream()` on the backend.
       *
       * Note that messages sent to the model do not appear in `.on('message')`
       * in this context.
       */
      static fromReadableStream(stream) {
        const runner = new _BetaMessageStream(null);
        runner._run(() => runner._fromReadableStream(stream));
        return runner;
      }
      static createMessage(messages, params, options, { logger } = {}) {
        const runner = new _BetaMessageStream(params, { logger });
        for (const message of params.messages) {
          runner._addMessageParam(message);
        }
        __classPrivateFieldSet(runner, _BetaMessageStream_params, { ...params, stream: true }, "f");
        runner._run(() => runner._createMessage(messages, { ...params, stream: true }, { ...options, headers: { ...options?.headers, [STAINLESS_HELPER_METHOD_HEADER]: "stream" } }));
        return runner;
      }
      _run(executor) {
        executor().then(() => {
          this._emitFinal();
          this._emit("end");
        }, __classPrivateFieldGet(this, _BetaMessageStream_handleError, "f"));
      }
      _addMessageParam(message) {
        this.messages.push(message);
      }
      _addMessage(message, emit = true) {
        this.receivedMessages.push(message);
        if (emit) {
          this._emit("message", message);
        }
      }
      async _createMessage(messages, params, options) {
        const signal = options?.signal;
        let abortHandler;
        if (signal) {
          if (signal.aborted)
            this.controller.abort();
          abortHandler = this.controller.abort.bind(this.controller);
          signal.addEventListener("abort", abortHandler);
        }
        try {
          __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_beginRequest).call(this);
          const { response, data: stream } = await messages.create({ ...params, stream: true }, { ...options, signal: this.controller.signal }).withResponse();
          this._connected(response);
          for await (const event of stream) {
            __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_addStreamEvent).call(this, event);
          }
          if (stream.controller.signal?.aborted) {
            throw new APIUserAbortError();
          }
          __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_endRequest).call(this);
        } finally {
          if (signal && abortHandler) {
            signal.removeEventListener("abort", abortHandler);
          }
        }
      }
      _connected(response) {
        if (this.ended)
          return;
        __classPrivateFieldSet(this, _BetaMessageStream_response, response, "f");
        __classPrivateFieldSet(this, _BetaMessageStream_request_id, response?.headers.get("request-id"), "f");
        __classPrivateFieldGet(this, _BetaMessageStream_resolveConnectedPromise, "f").call(this, response);
        this._emit("connect");
      }
      get ended() {
        return __classPrivateFieldGet(this, _BetaMessageStream_ended, "f");
      }
      get errored() {
        return __classPrivateFieldGet(this, _BetaMessageStream_errored, "f");
      }
      get aborted() {
        return __classPrivateFieldGet(this, _BetaMessageStream_aborted, "f");
      }
      abort() {
        this.controller.abort();
      }
      /**
       * Adds the listener function to the end of the listeners array for the event.
       * No checks are made to see if the listener has already been added. Multiple calls passing
       * the same combination of event and listener will result in the listener being added, and
       * called, multiple times.
       * @returns this MessageStream, so that calls can be chained
       */
      on(event, listener) {
        const listeners = __classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event] || (__classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event] = []);
        listeners.push({ listener });
        return this;
      }
      /**
       * Removes the specified listener from the listener array for the event.
       * off() will remove, at most, one instance of a listener from the listener array. If any single
       * listener has been added multiple times to the listener array for the specified event, then
       * off() must be called multiple times to remove each instance.
       * @returns this MessageStream, so that calls can be chained
       */
      off(event, listener) {
        const listeners = __classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event];
        if (!listeners)
          return this;
        const index = listeners.findIndex((l) => l.listener === listener);
        if (index >= 0)
          listeners.splice(index, 1);
        return this;
      }
      /**
       * Adds a one-time listener function for the event. The next time the event is triggered,
       * this listener is removed and then invoked.
       * @returns this MessageStream, so that calls can be chained
       */
      once(event, listener) {
        const listeners = __classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event] || (__classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event] = []);
        listeners.push({ listener, once: true });
        return this;
      }
      /**
       * This is similar to `.once()`, but returns a Promise that resolves the next time
       * the event is triggered, instead of calling a listener callback.
       * @returns a Promise that resolves the next time given event is triggered,
       * or rejects if an error is emitted.  (If you request the 'error' event,
       * returns a promise that resolves with the error).
       *
       * Example:
       *
       *   const message = await stream.emitted('message') // rejects if the stream errors
       */
      emitted(event) {
        return new Promise((resolve4, reject) => {
          __classPrivateFieldSet(this, _BetaMessageStream_catchingPromiseCreated, true, "f");
          if (event !== "error")
            this.once("error", reject);
          this.once(event, resolve4);
        });
      }
      async done() {
        __classPrivateFieldSet(this, _BetaMessageStream_catchingPromiseCreated, true, "f");
        await __classPrivateFieldGet(this, _BetaMessageStream_endPromise, "f");
      }
      get currentMessage() {
        return __classPrivateFieldGet(this, _BetaMessageStream_currentMessageSnapshot, "f");
      }
      /**
       * @returns a promise that resolves with the the final assistant Message response,
       * or rejects if an error occurred or the stream ended prematurely without producing a Message.
       * If structured outputs were used, this will be a ParsedMessage with a `parsed` field.
       */
      async finalMessage() {
        await this.done();
        return __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_getFinalMessage).call(this);
      }
      /**
       * @returns a promise that resolves with the the final assistant Message's text response, concatenated
       * together if there are more than one text blocks.
       * Rejects if an error occurred or the stream ended prematurely without producing a Message.
       */
      async finalText() {
        await this.done();
        return __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_getFinalText).call(this);
      }
      _emit(event, ...args) {
        if (__classPrivateFieldGet(this, _BetaMessageStream_ended, "f"))
          return;
        if (event === "end") {
          __classPrivateFieldSet(this, _BetaMessageStream_ended, true, "f");
          __classPrivateFieldGet(this, _BetaMessageStream_resolveEndPromise, "f").call(this);
        }
        const listeners = __classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event];
        if (listeners) {
          __classPrivateFieldGet(this, _BetaMessageStream_listeners, "f")[event] = listeners.filter((l) => !l.once);
          listeners.forEach(({ listener }) => listener(...args));
        }
        if (event === "abort") {
          const error = args[0];
          if (!__classPrivateFieldGet(this, _BetaMessageStream_catchingPromiseCreated, "f") && !listeners?.length) {
            Promise.reject(error);
          }
          __classPrivateFieldGet(this, _BetaMessageStream_rejectConnectedPromise, "f").call(this, error);
          __classPrivateFieldGet(this, _BetaMessageStream_rejectEndPromise, "f").call(this, error);
          this._emit("end");
          return;
        }
        if (event === "error") {
          const error = args[0];
          if (!__classPrivateFieldGet(this, _BetaMessageStream_catchingPromiseCreated, "f") && !listeners?.length) {
            Promise.reject(error);
          }
          __classPrivateFieldGet(this, _BetaMessageStream_rejectConnectedPromise, "f").call(this, error);
          __classPrivateFieldGet(this, _BetaMessageStream_rejectEndPromise, "f").call(this, error);
          this._emit("end");
        }
      }
      _emitFinal() {
        const finalMessage = this.receivedMessages.at(-1);
        if (finalMessage) {
          this._emit("finalMessage", __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_getFinalMessage).call(this));
        }
      }
      async _fromReadableStream(readableStream, options) {
        const signal = options?.signal;
        let abortHandler;
        if (signal) {
          if (signal.aborted)
            this.controller.abort();
          abortHandler = this.controller.abort.bind(this.controller);
          signal.addEventListener("abort", abortHandler);
        }
        try {
          __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_beginRequest).call(this);
          this._connected(null);
          const stream = Stream.fromReadableStream(readableStream, this.controller);
          for await (const event of stream) {
            __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_addStreamEvent).call(this, event);
          }
          if (stream.controller.signal?.aborted) {
            throw new APIUserAbortError();
          }
          __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_endRequest).call(this);
        } finally {
          if (signal && abortHandler) {
            signal.removeEventListener("abort", abortHandler);
          }
        }
      }
      [(_BetaMessageStream_currentMessageSnapshot = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_params = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_connectedPromise = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_resolveConnectedPromise = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_rejectConnectedPromise = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_endPromise = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_resolveEndPromise = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_rejectEndPromise = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_listeners = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_ended = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_errored = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_aborted = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_catchingPromiseCreated = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_response = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_request_id = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_logger = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_handleError = /* @__PURE__ */ new WeakMap(), _BetaMessageStream_instances = /* @__PURE__ */ new WeakSet(), _BetaMessageStream_getFinalMessage = function _BetaMessageStream_getFinalMessage2() {
        if (this.receivedMessages.length === 0) {
          throw new AnthropicError("stream ended without producing a Message with role=assistant");
        }
        return this.receivedMessages.at(-1);
      }, _BetaMessageStream_getFinalText = function _BetaMessageStream_getFinalText2() {
        if (this.receivedMessages.length === 0) {
          throw new AnthropicError("stream ended without producing a Message with role=assistant");
        }
        const textBlocks = this.receivedMessages.at(-1).content.filter((block) => block.type === "text").map((block) => block.text);
        if (textBlocks.length === 0) {
          throw new AnthropicError("stream ended without producing a content block with type=text");
        }
        return textBlocks.join(" ");
      }, _BetaMessageStream_beginRequest = function _BetaMessageStream_beginRequest2() {
        if (this.ended)
          return;
        __classPrivateFieldSet(this, _BetaMessageStream_currentMessageSnapshot, void 0, "f");
      }, _BetaMessageStream_addStreamEvent = function _BetaMessageStream_addStreamEvent2(event) {
        if (this.ended)
          return;
        const messageSnapshot = __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_accumulateMessage).call(this, event);
        this._emit("streamEvent", event, messageSnapshot);
        switch (event.type) {
          case "content_block_delta": {
            const content = messageSnapshot.content.at(-1);
            switch (event.delta.type) {
              case "text_delta": {
                if (content.type === "text") {
                  this._emit("text", event.delta.text, content.text || "");
                }
                break;
              }
              case "citations_delta": {
                if (content.type === "text") {
                  this._emit("citation", event.delta.citation, content.citations ?? []);
                }
                break;
              }
              case "input_json_delta": {
                if (tracksToolInput(content) && __classPrivateFieldGet(this, _BetaMessageStream_listeners, "f").inputJson?.length) {
                  let jsonSnapshot;
                  try {
                    jsonSnapshot = content.input;
                  } catch (err) {
                    __classPrivateFieldGet(this, _BetaMessageStream_handleError, "f").call(this, __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_toolInputParseError).call(this, content, err));
                    break;
                  }
                  this._emit("inputJson", event.delta.partial_json, jsonSnapshot);
                }
                break;
              }
              case "thinking_delta": {
                if (content.type === "thinking") {
                  this._emit("thinking", event.delta.thinking, content.thinking);
                }
                break;
              }
              case "signature_delta": {
                if (content.type === "thinking") {
                  this._emit("signature", content.signature);
                }
                break;
              }
              case "compaction_delta": {
                if (content.type === "compaction" && content.content) {
                  this._emit("compaction", content.content);
                }
                break;
              }
              default:
                checkNever(event.delta);
            }
            break;
          }
          case "message_stop": {
            this._addMessageParam(messageSnapshot);
            this._addMessage(maybeParseBetaMessage(messageSnapshot, __classPrivateFieldGet(this, _BetaMessageStream_params, "f"), { logger: __classPrivateFieldGet(this, _BetaMessageStream_logger, "f") }), true);
            break;
          }
          case "content_block_stop": {
            this._emit("contentBlock", messageSnapshot.content.at(-1));
            break;
          }
          case "message_start": {
            __classPrivateFieldSet(this, _BetaMessageStream_currentMessageSnapshot, messageSnapshot, "f");
            break;
          }
          case "content_block_start":
          case "message_delta":
            break;
        }
      }, _BetaMessageStream_endRequest = function _BetaMessageStream_endRequest2() {
        if (this.ended) {
          throw new AnthropicError(`stream has ended, this shouldn't happen`);
        }
        const snapshot = __classPrivateFieldGet(this, _BetaMessageStream_currentMessageSnapshot, "f");
        if (!snapshot) {
          throw new AnthropicError(`request ended without sending any chunks`);
        }
        __classPrivateFieldSet(this, _BetaMessageStream_currentMessageSnapshot, void 0, "f");
        return maybeParseBetaMessage(snapshot, __classPrivateFieldGet(this, _BetaMessageStream_params, "f"), { logger: __classPrivateFieldGet(this, _BetaMessageStream_logger, "f") });
      }, _BetaMessageStream_accumulateMessage = function _BetaMessageStream_accumulateMessage2(event) {
        let snapshot = __classPrivateFieldGet(this, _BetaMessageStream_currentMessageSnapshot, "f");
        if (event.type === "message_start") {
          if (snapshot) {
            throw new AnthropicError(`Unexpected event order, got ${event.type} before receiving "message_stop"`);
          }
          return event.message;
        }
        if (!snapshot) {
          throw new AnthropicError(`Unexpected event order, got ${event.type} before "message_start"`);
        }
        switch (event.type) {
          case "message_stop":
            return snapshot;
          case "message_delta":
            snapshot.container = event.delta.container;
            snapshot.stop_reason = event.delta.stop_reason;
            snapshot.stop_sequence = event.delta.stop_sequence;
            if (event.delta.stop_details != null) {
              snapshot.stop_details = event.delta.stop_details;
            }
            snapshot.usage.output_tokens = event.usage.output_tokens;
            snapshot.context_management = event.context_management;
            if (event.usage.input_tokens != null) {
              snapshot.usage.input_tokens = event.usage.input_tokens;
            }
            if (event.usage.cache_creation_input_tokens != null) {
              snapshot.usage.cache_creation_input_tokens = event.usage.cache_creation_input_tokens;
            }
            if (event.usage.cache_read_input_tokens != null) {
              snapshot.usage.cache_read_input_tokens = event.usage.cache_read_input_tokens;
            }
            if (event.usage.server_tool_use != null) {
              snapshot.usage.server_tool_use = event.usage.server_tool_use;
            }
            if (event.usage.iterations != null) {
              snapshot.usage.iterations = event.usage.iterations;
            }
            if (event.usage.fallback_credit != null) {
              snapshot.usage.fallback_credit = event.usage.fallback_credit;
            }
            return snapshot;
          case "content_block_start":
            snapshot.content.push(event.content_block);
            if (event.content_block.type === "fallback") {
              snapshot.model = event.content_block.to.model;
            }
            return snapshot;
          case "content_block_delta": {
            const snapshotContent = snapshot.content.at(event.index);
            switch (event.delta.type) {
              case "text_delta": {
                if (snapshotContent?.type === "text") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    text: (snapshotContent.text || "") + event.delta.text
                  };
                }
                break;
              }
              case "citations_delta": {
                if (snapshotContent?.type === "text") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    citations: [...snapshotContent.citations ?? [], event.delta.citation]
                  };
                }
                break;
              }
              case "input_json_delta": {
                if (snapshotContent && tracksToolInput(snapshotContent)) {
                  const jsonBuf = (snapshotContent[JSON_BUF_PROPERTY] || "") + event.delta.partial_json;
                  snapshot.content[event.index] = withLazyInput(snapshotContent, jsonBuf);
                }
                break;
              }
              case "thinking_delta": {
                if (snapshotContent?.type === "thinking") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    thinking: snapshotContent.thinking + event.delta.thinking
                  };
                }
                break;
              }
              case "signature_delta": {
                if (snapshotContent?.type === "thinking") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    signature: event.delta.signature
                  };
                }
                break;
              }
              case "compaction_delta": {
                if (snapshotContent?.type === "compaction") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    content: (snapshotContent.content || "") + event.delta.content,
                    encrypted_content: event.delta.encrypted_content
                  };
                }
                break;
              }
              default:
                checkNever(event.delta);
            }
            return snapshot;
          }
          case "content_block_stop": {
            const snapshotContent = snapshot.content.at(event.index);
            if (snapshotContent && tracksToolInput(snapshotContent) && JSON_BUF_PROPERTY in snapshotContent) {
              let input;
              try {
                input = snapshotContent.input;
              } catch (err) {
                input = {};
                __classPrivateFieldGet(this, _BetaMessageStream_handleError, "f").call(this, __classPrivateFieldGet(this, _BetaMessageStream_instances, "m", _BetaMessageStream_toolInputParseError).call(this, snapshotContent, err));
              }
              Object.defineProperty(snapshotContent, "input", {
                value: input,
                enumerable: true,
                configurable: true,
                writable: true
              });
            }
            return snapshot;
          }
        }
      }, _BetaMessageStream_toolInputParseError = function _BetaMessageStream_toolInputParseError2(block, err) {
        const jsonBuf = block[JSON_BUF_PROPERTY];
        return new AnthropicError(`Unable to parse tool parameter JSON from model. Please retry your request or adjust your prompt. Error: ${err}. JSON: ${jsonBuf}`);
      }, Symbol.asyncIterator)]() {
        const pushQueue = [];
        const readQueue = [];
        let done = false;
        this.on("streamEvent", (event) => {
          const reader = readQueue.shift();
          if (reader) {
            reader.resolve(event);
          } else {
            pushQueue.push(event);
          }
        });
        this.on("end", () => {
          done = true;
          for (const reader of readQueue) {
            reader.resolve(void 0);
          }
          readQueue.length = 0;
        });
        this.on("abort", (err) => {
          done = true;
          for (const reader of readQueue) {
            reader.reject(err);
          }
          readQueue.length = 0;
        });
        this.on("error", (err) => {
          done = true;
          for (const reader of readQueue) {
            reader.reject(err);
          }
          readQueue.length = 0;
        });
        return {
          next: async () => {
            if (!pushQueue.length) {
              if (done) {
                return { value: void 0, done: true };
              }
              return new Promise((resolve4, reject) => readQueue.push({ resolve: resolve4, reject })).then((chunk2) => chunk2 ? { value: chunk2, done: false } : { value: void 0, done: true });
            }
            const chunk = pushQueue.shift();
            return { value: chunk, done: false };
          },
          return: async () => {
            this.abort();
            return { value: void 0, done: true };
          }
        };
      }
      toReadableStream() {
        const stream = new Stream(this[Symbol.asyncIterator].bind(this), this.controller);
        return stream.toReadableStream();
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/lib/tools/CompactionControl.mjs
var DEFAULT_TOKEN_THRESHOLD, DEFAULT_SUMMARY_PROMPT;
var init_CompactionControl = __esm({
  "node_modules/@anthropic-ai/sdk/lib/tools/CompactionControl.mjs"() {
    DEFAULT_TOKEN_THRESHOLD = 1e5;
    DEFAULT_SUMMARY_PROMPT = `You have been working on the task described above but have not yet completed it. Write a continuation summary that will allow you (or another instance of yourself) to resume work efficiently in a future context window where the conversation history will be replaced with this summary. Your summary should be structured, concise, and actionable. Include:
1. Task Overview
The user's core request and success criteria
Any clarifications or constraints they specified
2. Current State
What has been completed so far
Files created, modified, or analyzed (with paths if relevant)
Key outputs or artifacts produced
3. Important Discoveries
Technical constraints or requirements uncovered
Decisions made and their rationale
Errors encountered and how they were resolved
What approaches were tried that didn't work (and why)
4. Next Steps
Specific actions needed to complete the task
Any blockers or open questions to resolve
Priority order if multiple steps remain
5. Context to Preserve
User preferences or style requirements
Domain-specific details that aren't obvious
Any promises made to the user
Be concise but complete\u2014err on the side of including information that would prevent duplicate work or repeated mistakes. Write in a way that enables immediate resumption of the task.
Wrap your summary in <summary></summary> tags.`;
  }
});

// node_modules/@anthropic-ai/sdk/lib/tools/BetaToolRunner.mjs
async function generateToolResponse(params, lastMessage = params.messages.at(-1), requestOptions) {
  if (!lastMessage || lastMessage.role !== "assistant" || !lastMessage.content || typeof lastMessage.content === "string") {
    return null;
  }
  const toolUseBlocks = lastMessage.content.filter((content) => content.type === "tool_use");
  if (toolUseBlocks.length === 0) {
    return null;
  }
  const available = availableToolNames(params);
  const toolResults = await Promise.all(toolUseBlocks.map(async (toolUse) => {
    const tool = params.tools.find((t) => ("name" in t ? t.name : t.mcp_server_name) === toolUse.name);
    if (!tool || !("run" in tool) || !available.has(toolUse.name)) {
      return toolNotFoundResult(toolUse);
    }
    try {
      let input = toolUse.input;
      if ("parse" in tool && tool.parse) {
        input = tool.parse(input);
      }
      const result = await tool.run(input, {
        toolUse,
        toolUseBlock: toolUse,
        signal: requestOptions?.signal
      });
      return {
        type: "tool_result",
        tool_use_id: toolUse.id,
        content: result
      };
    } catch (error) {
      return {
        type: "tool_result",
        tool_use_id: toolUse.id,
        content: error instanceof ToolError ? error.content : `Error: ${error instanceof Error ? error.message : String(error)}`,
        is_error: true
      };
    }
  }));
  return {
    role: "user",
    content: toolResults
  };
}
function toolNotFoundResult(toolUse) {
  return {
    type: "tool_result",
    tool_use_id: toolUse.id,
    content: `Error: Tool '${toolUse.name}' not found`,
    is_error: true
  };
}
function availableToolNames(params) {
  const available = /* @__PURE__ */ new Set();
  for (const tool of params.tools) {
    if ("run" in tool) {
      available.add(tool.name);
    }
  }
  for (const message of params.messages) {
    if (message.role !== "system" || typeof message.content === "string") {
      continue;
    }
    for (const block of message.content) {
      applyToolChange(block, available);
    }
  }
  return available;
}
function applyToolChange(block, available) {
  switch (block.type) {
    case "tool_removal":
    case "tool_addition":
      applyToolReference(block, available);
      break;
    case "mid_conv_system":
      for (const inner of block.content) {
        if (inner.type === "tool_removal" || inner.type === "tool_addition") {
          applyToolReference(inner, available);
        }
      }
      break;
    default:
      break;
  }
}
function applyToolReference(block, available) {
  const name = referencedToolName(block.tool);
  if (name === void 0)
    return;
  if (block.type === "tool_removal") {
    available.delete(name);
  } else {
    available.add(name);
  }
}
function referencedToolName(ref) {
  switch (ref.type) {
    case "tool_reference":
      return ref.name;
    default:
      return void 0;
  }
}
var _BetaToolRunner_instances, _BetaToolRunner_consumed, _BetaToolRunner_mutated, _BetaToolRunner_state, _BetaToolRunner_options, _BetaToolRunner_message, _BetaToolRunner_toolResponse, _BetaToolRunner_completion, _BetaToolRunner_iterationCount, _BetaToolRunner_checkAndCompact, _BetaToolRunner_generateToolResponse, BetaToolRunner;
var init_BetaToolRunner = __esm({
  "node_modules/@anthropic-ai/sdk/lib/tools/BetaToolRunner.mjs"() {
    init_tslib();
    init_ToolError();
    init_error();
    init_headers();
    init_promise();
    init_CompactionControl();
    init_stainless_helper_header();
    BetaToolRunner = class {
      constructor(client, params, options) {
        _BetaToolRunner_instances.add(this);
        this.client = client;
        _BetaToolRunner_consumed.set(this, false);
        _BetaToolRunner_mutated.set(this, false);
        _BetaToolRunner_state.set(this, void 0);
        _BetaToolRunner_options.set(this, void 0);
        _BetaToolRunner_message.set(this, void 0);
        _BetaToolRunner_toolResponse.set(this, void 0);
        _BetaToolRunner_completion.set(this, void 0);
        _BetaToolRunner_iterationCount.set(this, 0);
        __classPrivateFieldSet(this, _BetaToolRunner_state, {
          params: {
            // You can't clone the entire params since there are functions as handlers.
            // You also don't really need to clone params.messages, but it probably will prevent a foot gun
            // somewhere.
            ...params,
            messages: structuredClone(params.messages)
          }
        }, "f");
        const collected = collectStainlessHelpers(params.tools, params.messages);
        __classPrivateFieldSet(this, _BetaToolRunner_options, {
          ...options,
          headers: buildHeaders([
            helperHeader("BetaToolRunner"),
            collected.length ? { [STAINLESS_HELPER_HEADER]: collected.join(", ") } : void 0,
            options?.headers
          ])
        }, "f");
        __classPrivateFieldSet(this, _BetaToolRunner_completion, promiseWithResolvers(), "f");
        if (params.compactionControl?.enabled) {
          console.warn('Anthropic: The `compactionControl` parameter is deprecated and will be removed in a future version. Use server-side compaction instead by passing `edits: [{ type: "compact_20260112" }]` in the params passed to `toolRunner()`. See https://platform.claude.com/docs/en/build-with-claude/compaction');
        }
      }
      async *[(_BetaToolRunner_consumed = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_mutated = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_state = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_options = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_message = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_toolResponse = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_completion = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_iterationCount = /* @__PURE__ */ new WeakMap(), _BetaToolRunner_instances = /* @__PURE__ */ new WeakSet(), _BetaToolRunner_checkAndCompact = async function _BetaToolRunner_checkAndCompact2() {
        const compactionControl = __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.compactionControl;
        if (!compactionControl || !compactionControl.enabled) {
          return false;
        }
        let tokensUsed = 0;
        if (__classPrivateFieldGet(this, _BetaToolRunner_message, "f") !== void 0) {
          try {
            const message = await __classPrivateFieldGet(this, _BetaToolRunner_message, "f");
            const totalInputTokens = message.usage.input_tokens + (message.usage.cache_creation_input_tokens ?? 0) + (message.usage.cache_read_input_tokens ?? 0);
            tokensUsed = totalInputTokens + message.usage.output_tokens;
          } catch {
            return false;
          }
        }
        const threshold = compactionControl.contextTokenThreshold ?? DEFAULT_TOKEN_THRESHOLD;
        if (tokensUsed < threshold) {
          return false;
        }
        const model = compactionControl.model ?? __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.model;
        const summaryPrompt = compactionControl.summaryPrompt ?? DEFAULT_SUMMARY_PROMPT;
        const messages = __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.messages;
        if (messages[messages.length - 1].role === "assistant") {
          const lastMessage = messages[messages.length - 1];
          if (Array.isArray(lastMessage.content)) {
            const nonToolBlocks = lastMessage.content.filter((block) => block.type !== "tool_use");
            if (nonToolBlocks.length === 0) {
              messages.pop();
            } else {
              lastMessage.content = nonToolBlocks;
            }
          }
        }
        const response = await this.client.beta.messages.create({
          model,
          messages: [
            ...messages,
            {
              role: "user",
              content: [
                {
                  type: "text",
                  text: summaryPrompt
                }
              ]
            }
          ],
          max_tokens: __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.max_tokens
        }, {
          signal: __classPrivateFieldGet(this, _BetaToolRunner_options, "f").signal,
          headers: buildHeaders([__classPrivateFieldGet(this, _BetaToolRunner_options, "f").headers, helperHeader("compaction")])
        });
        if (response.content[0]?.type !== "text") {
          throw new AnthropicError("Expected text response for compaction");
        }
        __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.messages = [
          {
            role: "user",
            content: response.content
          }
        ];
        return true;
      }, Symbol.asyncIterator)]() {
        var _a2;
        if (__classPrivateFieldGet(this, _BetaToolRunner_consumed, "f")) {
          throw new AnthropicError("Cannot iterate over a consumed stream");
        }
        __classPrivateFieldSet(this, _BetaToolRunner_consumed, true, "f");
        __classPrivateFieldSet(this, _BetaToolRunner_mutated, true, "f");
        __classPrivateFieldSet(this, _BetaToolRunner_toolResponse, void 0, "f");
        try {
          while (true) {
            let stream;
            try {
              if (__classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.max_iterations && __classPrivateFieldGet(this, _BetaToolRunner_iterationCount, "f") >= __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.max_iterations) {
                break;
              }
              __classPrivateFieldSet(this, _BetaToolRunner_mutated, false, "f");
              __classPrivateFieldSet(this, _BetaToolRunner_toolResponse, void 0, "f");
              __classPrivateFieldSet(this, _BetaToolRunner_iterationCount, (_a2 = __classPrivateFieldGet(this, _BetaToolRunner_iterationCount, "f"), _a2++, _a2), "f");
              __classPrivateFieldSet(this, _BetaToolRunner_message, void 0, "f");
              const { max_iterations, compactionControl, ...params } = __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params;
              if (params.stream) {
                stream = this.client.beta.messages.stream({ ...params }, __classPrivateFieldGet(this, _BetaToolRunner_options, "f"));
                __classPrivateFieldSet(this, _BetaToolRunner_message, stream.finalMessage(), "f");
                __classPrivateFieldGet(this, _BetaToolRunner_message, "f").catch(() => {
                });
                yield stream;
              } else {
                __classPrivateFieldSet(this, _BetaToolRunner_message, this.client.beta.messages.create({ ...params, stream: false }, __classPrivateFieldGet(this, _BetaToolRunner_options, "f")), "f");
                yield __classPrivateFieldGet(this, _BetaToolRunner_message, "f");
              }
              const isCompacted = await __classPrivateFieldGet(this, _BetaToolRunner_instances, "m", _BetaToolRunner_checkAndCompact).call(this);
              if (!isCompacted) {
                if (!__classPrivateFieldGet(this, _BetaToolRunner_mutated, "f")) {
                  const message = await __classPrivateFieldGet(this, _BetaToolRunner_message, "f");
                  __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.messages.push({ role: message.role, content: message.content });
                  if (message.stop_reason === "refusal") {
                    break;
                  }
                }
                const toolMessage = await __classPrivateFieldGet(this, _BetaToolRunner_instances, "m", _BetaToolRunner_generateToolResponse).call(this, __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.messages.at(-1));
                if (toolMessage) {
                  __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params.messages.push(toolMessage);
                } else if (!__classPrivateFieldGet(this, _BetaToolRunner_mutated, "f")) {
                  break;
                }
              }
            } finally {
              if (stream) {
                stream.abort();
              }
            }
          }
          if (!__classPrivateFieldGet(this, _BetaToolRunner_message, "f")) {
            throw new AnthropicError("ToolRunner concluded without a message from the server");
          }
          __classPrivateFieldGet(this, _BetaToolRunner_completion, "f").resolve(await __classPrivateFieldGet(this, _BetaToolRunner_message, "f"));
        } catch (error) {
          __classPrivateFieldSet(this, _BetaToolRunner_consumed, false, "f");
          __classPrivateFieldGet(this, _BetaToolRunner_completion, "f").promise.catch(() => {
          });
          __classPrivateFieldGet(this, _BetaToolRunner_completion, "f").reject(error);
          __classPrivateFieldSet(this, _BetaToolRunner_completion, promiseWithResolvers(), "f");
          throw error;
        }
      }
      setMessagesParams(paramsOrMutator) {
        if (typeof paramsOrMutator === "function") {
          __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params = paramsOrMutator(__classPrivateFieldGet(this, _BetaToolRunner_state, "f").params);
        } else {
          __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params = paramsOrMutator;
        }
        __classPrivateFieldSet(this, _BetaToolRunner_mutated, true, "f");
        __classPrivateFieldSet(this, _BetaToolRunner_toolResponse, void 0, "f");
      }
      setRequestOptions(optionsOrMutator) {
        if (typeof optionsOrMutator === "function") {
          __classPrivateFieldSet(this, _BetaToolRunner_options, optionsOrMutator(__classPrivateFieldGet(this, _BetaToolRunner_options, "f")), "f");
        } else {
          __classPrivateFieldSet(this, _BetaToolRunner_options, { ...__classPrivateFieldGet(this, _BetaToolRunner_options, "f"), ...optionsOrMutator }, "f");
        }
      }
      /**
       * Get the tool response for the last message from the assistant.
       * Avoids redundant tool executions by caching results.
       *
       * @returns A promise that resolves to a BetaMessageParam containing tool results, or null if no tools need to be executed
       *
       * @example
       * const toolResponse = await runner.generateToolResponse();
       * if (toolResponse) {
       *   console.log('Tool results:', toolResponse.content);
       * }
       */
      async generateToolResponse(signal = __classPrivateFieldGet(this, _BetaToolRunner_options, "f").signal) {
        const message = await __classPrivateFieldGet(this, _BetaToolRunner_message, "f") ?? this.params.messages.at(-1);
        if (!message) {
          return null;
        }
        return __classPrivateFieldGet(this, _BetaToolRunner_instances, "m", _BetaToolRunner_generateToolResponse).call(this, message, signal);
      }
      /**
       * Wait for the async iterator to complete. This works even if the async iterator hasn't yet started, and
       * will wait for an instance to start and go to completion.
       *
       * @returns A promise that resolves to the final BetaMessage when the iterator completes
       *
       * @example
       * // Start consuming the iterator
       * for await (const message of runner) {
       *   console.log('Message:', message.content);
       * }
       *
       * // Meanwhile, wait for completion from another part of the code
       * const finalMessage = await runner.done();
       * console.log('Final response:', finalMessage.content);
       */
      done() {
        return __classPrivateFieldGet(this, _BetaToolRunner_completion, "f").promise;
      }
      /**
       * Returns a promise indicating that the stream is done. Unlike .done(), this will eagerly read the stream:
       * * If the iterator has not been consumed, consume the entire iterator and return the final message from the
       * assistant.
       * * If the iterator has been consumed, waits for it to complete and returns the final message.
       *
       * @returns A promise that resolves to the final BetaMessage from the conversation
       * @throws {AnthropicError} If no messages were processed during the conversation
       *
       * @example
       * const finalMessage = await runner.runUntilDone();
       * console.log('Final response:', finalMessage.content);
       */
      async runUntilDone() {
        if (!__classPrivateFieldGet(this, _BetaToolRunner_consumed, "f")) {
          for await (const _ of this) {
          }
        }
        return this.done();
      }
      /**
       * Get the current parameters being used by the ToolRunner.
       *
       * @returns A readonly view of the current ToolRunnerParams
       *
       * @example
       * const currentParams = runner.params;
       * console.log('Current model:', currentParams.model);
       * console.log('Message count:', currentParams.messages.length);
       */
      get params() {
        return __classPrivateFieldGet(this, _BetaToolRunner_state, "f").params;
      }
      /**
       * Add one or more messages to the conversation history.
       *
       * @param messages - One or more BetaMessageParam objects to add to the conversation
       *
       * @example
       * runner.pushMessages(
       *   { role: 'user', content: 'Also, what about the weather in NYC?' }
       * );
       *
       * @example
       * // Adding multiple messages
       * runner.pushMessages(
       *   { role: 'user', content: 'What about NYC?' },
       *   { role: 'user', content: 'And Boston?' }
       * );
       */
      pushMessages(...messages) {
        this.setMessagesParams((params) => ({
          ...params,
          messages: [...params.messages, ...messages]
        }));
      }
      /**
       * Makes the ToolRunner directly awaitable, equivalent to calling .runUntilDone()
       * This allows using `await runner` instead of `await runner.runUntilDone()`
       */
      then(onfulfilled, onrejected) {
        return this.runUntilDone().then(onfulfilled, onrejected);
      }
    };
    _BetaToolRunner_generateToolResponse = async function _BetaToolRunner_generateToolResponse2(lastMessage, signal = __classPrivateFieldGet(this, _BetaToolRunner_options, "f").signal) {
      if (__classPrivateFieldGet(this, _BetaToolRunner_toolResponse, "f") !== void 0) {
        return __classPrivateFieldGet(this, _BetaToolRunner_toolResponse, "f");
      }
      __classPrivateFieldSet(this, _BetaToolRunner_toolResponse, generateToolResponse(__classPrivateFieldGet(this, _BetaToolRunner_state, "f").params, lastMessage, {
        ...__classPrivateFieldGet(this, _BetaToolRunner_options, "f"),
        signal
      }), "f");
      return __classPrivateFieldGet(this, _BetaToolRunner_toolResponse, "f");
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/messages/messages.mjs
function transformOutputFormat(params) {
  if (!params.output_format) {
    return params;
  }
  if (params.output_config?.format) {
    throw new AnthropicError("Both output_format and output_config.format were provided. Please use only output_config.format (output_format is deprecated).");
  }
  const { output_format, ...rest } = params;
  return {
    ...rest,
    output_config: {
      ...params.output_config,
      format: output_format
    }
  };
}
var DEPRECATED_MODELS, MODELS_TO_WARN_WITH_THINKING_ENABLED, Messages;
var init_messages = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/messages/messages.mjs"() {
    init_error2();
    init_batches();
    init_resource();
    init_constants();
    init_headers();
    init_stainless_helper_header();
    init_beta_parser();
    init_BetaMessageStream();
    init_BetaToolRunner();
    init_ToolError();
    init_batches();
    init_BetaToolRunner();
    init_ToolError();
    DEPRECATED_MODELS = {};
    MODELS_TO_WARN_WITH_THINKING_ENABLED = ["claude-mythos-preview", "claude-opus-4-6"];
    Messages = class extends APIResource {
      constructor() {
        super(...arguments);
        this.batches = new Batches(this._client);
      }
      create(params, options) {
        const modifiedParams = transformOutputFormat(params);
        const { betas, user_profile_id, ...body } = modifiedParams;
        if (body.model in DEPRECATED_MODELS) {
          console.warn(`The model '${body.model}' is deprecated and will reach end-of-life on ${DEPRECATED_MODELS[body.model]}
Please migrate to a newer model. Visit https://docs.anthropic.com/en/docs/resources/model-deprecations for more information.`);
        }
        if (MODELS_TO_WARN_WITH_THINKING_ENABLED.includes(body.model) && body.thinking && body.thinking.type === "enabled") {
          console.warn(`Using Claude with ${body.model} and 'thinking.type=enabled' is deprecated. Use 'thinking.type=adaptive' instead which results in better model performance in our testing: https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking`);
        }
        let timeout = this._client._options.timeout;
        if (!body.stream && timeout == null) {
          const maxNonstreamingTokens = MODEL_NONSTREAMING_TOKENS[body.model] ?? void 0;
          timeout = this._client.calculateNonstreamingTimeout(body.max_tokens, maxNonstreamingTokens);
        }
        const helperHeader2 = stainlessHelperHeader(body.tools, body.messages);
        return this._client.post("/v1/messages?beta=true", {
          body,
          timeout: timeout ?? 6e5,
          ...options,
          headers: buildHeaders([
            {
              ...betas?.toString() != null ? { "anthropic-beta": betas?.toString() } : void 0,
              ...user_profile_id != null ? { "anthropic-user-profile-id": user_profile_id } : void 0
            },
            helperHeader2,
            options?.headers
          ]),
          stream: modifiedParams.stream ?? false
        });
      }
      /**
       * Send a structured list of input messages with text and/or image content, along with an expected `output_format` and
       * the response will be automatically parsed and available in the `parsed_output` property of the message.
       *
       * @example
       * ```ts
       * const message = await client.beta.messages.parse({
       *   model: 'claude-3-5-sonnet-20241022',
       *   max_tokens: 1024,
       *   messages: [{ role: 'user', content: 'What is 2+2?' }],
       *   output_format: zodOutputFormat(z.object({ answer: z.number() }), 'math'),
       * });
       *
       * console.log(message.parsed_output?.answer); // 4
       * ```
       */
      parse(params, options) {
        options = {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...params.betas ?? [], "structured-outputs-2025-12-15"].toString() },
            options?.headers
          ])
        };
        return this.create(params, options).then((message) => parseBetaMessage(message, params, { logger: this._client.logger ?? console }));
      }
      /**
       * Create a Message stream
       */
      stream(body, options) {
        return BetaMessageStream.createMessage(this, body, options);
      }
      /**
       * Count the number of tokens in a Message.
       *
       * The Token Count API can be used to count the number of tokens in a Message,
       * including tools, images, and documents, without creating it.
       *
       * Learn more about token counting in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/token-counting)
       *
       * @example
       * ```ts
       * const betaMessageTokensCount =
       *   await client.beta.messages.countTokens({
       *     messages: [{ content: 'Hello, world', role: 'user' }],
       *     model: 'claude-opus-4-6',
       *   });
       * ```
       */
      countTokens(params, options) {
        const modifiedParams = transformOutputFormat(params);
        const { betas, user_profile_id, ...body } = modifiedParams;
        return this._client.post("/v1/messages/count_tokens?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            {
              "anthropic-beta": [...betas ?? [], "token-counting-2024-11-01"].toString(),
              ...user_profile_id != null ? { "anthropic-user-profile-id": user_profile_id } : void 0
            },
            options?.headers
          ])
        });
      }
      toolRunner(body, options) {
        return new BetaToolRunner(this._client, body, options);
      }
    };
    Messages.Batches = Batches;
    Messages.BetaToolRunner = BetaToolRunner;
    Messages.ToolError = ToolError;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/sessions/events.mjs
var Events;
var init_events = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/sessions/events.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    init_SessionToolRunner();
    init_SessionToolRunner();
    Events = class extends APIResource {
      /**
       * List Events
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsSessionEvent of client.beta.sessions.events.list(
       *   'sesn_011CZkZAtmR3yMPDzynEDxu7',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(sessionID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/sessions/${sessionID}/events?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Send Events
       *
       * @example
       * ```ts
       * const betaManagedAgentsSendSessionEvents =
       *   await client.beta.sessions.events.send(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *     {
       *       events: [
       *         {
       *           content: [
       *             {
       *               text: 'Where is my order #1234?',
       *               type: 'text',
       *             },
       *           ],
       *           type: 'user.message',
       *         },
       *       ],
       *     },
       *   );
       * ```
       */
      send(sessionID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/sessions/${sessionID}/events?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Stream Events
       *
       * @example
       * ```ts
       * const betaManagedAgentsStreamSessionEvents =
       *   await client.beta.sessions.events.stream(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *   );
       * ```
       */
      stream(sessionID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.get(path`/v1/sessions/${sessionID}/events/stream?beta=true`, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ]),
          stream: true
        });
      }
      /**
       * Attach to a session and dispatch every incoming `agent.tool_use` and
       * `agent.custom_tool_use` event to a local tool registry, sending the matching
       * result back (`user.tool_result` / `user.custom_tool_result`). The
       * sessions-side counterpart to `client.beta.messages.toolRunner`: yields one
       * entry per completed tool call so callers can observe each dispatch (and
       * `break` to abort cleanly).
       *
       * @example
       * ```ts
       * import { betaAgentToolset20260401 } from '@anthropic-ai/sdk/tools/agent-toolset/node';
       *
       * for await (const call of client.beta.sessions.events.toolRunner(work.data.id, {
       *   tools: [...betaAgentToolset20260401({ workdir }), myTool],
       * })) {
       *   console.log(`${call.name} -> ${call.isError ? 'error' : 'ok'}`);
       * }
       * ```
       */
      toolRunner(sessionID, opts) {
        return new SessionToolRunner(sessionID, { ...opts, client: this._client });
      }
    };
    Events.SessionToolRunner = SessionToolRunner;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/sessions/resources.mjs
var Resources;
var init_resources = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/sessions/resources.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Resources = class extends APIResource {
      /**
       * Get Session Resource
       *
       * @example
       * ```ts
       * const resource =
       *   await client.beta.sessions.resources.retrieve(
       *     'sesrsc_011CZkZBJq5dWxk9fVLNcPht',
       *     { session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7' },
       *   );
       * ```
       */
      retrieve(resourceID, params, options) {
        const { session_id, betas } = params;
        return this._client.get(path`/v1/sessions/${session_id}/resources/${resourceID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update Session Resource
       *
       * @example
       * ```ts
       * const resource =
       *   await client.beta.sessions.resources.update(
       *     'sesrsc_011CZkZBJq5dWxk9fVLNcPht',
       *     {
       *       session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *       authorization_token: 'ghp_exampletoken',
       *     },
       *   );
       * ```
       */
      update(resourceID, params, options) {
        const { session_id, betas, ...body } = params;
        return this._client.post(path`/v1/sessions/${session_id}/resources/${resourceID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Session Resources
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsSessionResource of client.beta.sessions.resources.list(
       *   'sesn_011CZkZAtmR3yMPDzynEDxu7',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(sessionID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/sessions/${sessionID}/resources?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete Session Resource
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeleteSessionResource =
       *   await client.beta.sessions.resources.delete(
       *     'sesrsc_011CZkZBJq5dWxk9fVLNcPht',
       *     { session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7' },
       *   );
       * ```
       */
      delete(resourceID, params, options) {
        const { session_id, betas } = params;
        return this._client.delete(path`/v1/sessions/${session_id}/resources/${resourceID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Add Session Resource
       *
       * @example
       * ```ts
       * const betaManagedAgentsFileResource =
       *   await client.beta.sessions.resources.add(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *     {
       *       file_id: 'file_011CNha8iCJcU1wXNR6q4V8w',
       *       type: 'file',
       *     },
       *   );
       * ```
       */
      add(sessionID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/sessions/${sessionID}/resources?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/sessions/threads/events.mjs
var Events2;
var init_events2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/sessions/threads/events.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Events2 = class extends APIResource {
      /**
       * List Session Thread Events
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsSessionEvent of client.beta.sessions.threads.events.list(
       *   'sthr_011CZkZVWa6oIjw0rgXZpnBt',
       *   { session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7' },
       * )) {
       *   // ...
       * }
       * ```
       */
      list(threadID, params, options) {
        const { session_id, betas, ...query } = params;
        return this._client.getAPIList(path`/v1/sessions/${session_id}/threads/${threadID}/events?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Stream Session Thread Events
       *
       * @example
       * ```ts
       * const betaManagedAgentsStreamSessionThreadEvents =
       *   await client.beta.sessions.threads.events.stream(
       *     'sthr_011CZkZVWa6oIjw0rgXZpnBt',
       *     { session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7' },
       *   );
       * ```
       */
      stream(threadID, params, options) {
        const { session_id, betas, ...query } = params;
        return this._client.get(path`/v1/sessions/${session_id}/threads/${threadID}/stream?beta=true`, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ]),
          stream: true
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/sessions/threads/threads.mjs
var Threads;
var init_threads = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/sessions/threads/threads.mjs"() {
    init_resource();
    init_events2();
    init_events2();
    init_pagination();
    init_headers();
    init_path();
    Threads = class extends APIResource {
      constructor() {
        super(...arguments);
        this.events = new Events2(this._client);
      }
      /**
       * Get Session Thread
       *
       * @example
       * ```ts
       * const betaManagedAgentsSessionThread =
       *   await client.beta.sessions.threads.retrieve(
       *     'sthr_011CZkZVWa6oIjw0rgXZpnBt',
       *     { session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7' },
       *   );
       * ```
       */
      retrieve(threadID, params, options) {
        const { session_id, betas } = params;
        return this._client.get(path`/v1/sessions/${session_id}/threads/${threadID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Session Threads
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsSessionThread of client.beta.sessions.threads.list(
       *   'sesn_011CZkZAtmR3yMPDzynEDxu7',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(sessionID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/sessions/${sessionID}/threads?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive Session Thread
       *
       * @example
       * ```ts
       * const betaManagedAgentsSessionThread =
       *   await client.beta.sessions.threads.archive(
       *     'sthr_011CZkZVWa6oIjw0rgXZpnBt',
       *     { session_id: 'sesn_011CZkZAtmR3yMPDzynEDxu7' },
       *   );
       * ```
       */
      archive(threadID, params, options) {
        const { session_id, betas } = params;
        return this._client.post(path`/v1/sessions/${session_id}/threads/${threadID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
    Threads.Events = Events2;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/sessions/sessions.mjs
var Sessions;
var init_sessions = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/sessions/sessions.mjs"() {
    init_resource();
    init_events();
    init_events();
    init_resources();
    init_resources();
    init_threads();
    init_threads();
    init_pagination();
    init_headers();
    init_path();
    Sessions = class extends APIResource {
      constructor() {
        super(...arguments);
        this.events = new Events(this._client);
        this.resources = new Resources(this._client);
        this.threads = new Threads(this._client);
      }
      /**
       * Create Session
       *
       * @example
       * ```ts
       * const betaManagedAgentsSession =
       *   await client.beta.sessions.create({
       *     agent: 'agent_011CZkYpogX7uDKUyvBTophP',
       *     environment_id: 'env_011CZkZ9X2dpNyB7HsEFoRfW',
       *   });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/sessions?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get Session
       *
       * @example
       * ```ts
       * const betaManagedAgentsSession =
       *   await client.beta.sessions.retrieve(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *   );
       * ```
       */
      retrieve(sessionID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/sessions/${sessionID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update Session
       *
       * @example
       * ```ts
       * const betaManagedAgentsSession =
       *   await client.beta.sessions.update(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *   );
       * ```
       */
      update(sessionID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/sessions/${sessionID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Sessions
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsSession of client.beta.sessions.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/sessions?beta=true", BidirectionalPageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete Session
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeletedSession =
       *   await client.beta.sessions.delete(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *   );
       * ```
       */
      delete(sessionID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/sessions/${sessionID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive Session
       *
       * @example
       * ```ts
       * const betaManagedAgentsSession =
       *   await client.beta.sessions.archive(
       *     'sesn_011CZkZAtmR3yMPDzynEDxu7',
       *   );
       * ```
       */
      archive(sessionID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/sessions/${sessionID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
    Sessions.Events = Events;
    Sessions.Resources = Resources;
    Sessions.Threads = Threads;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/skills/versions.mjs
var Versions2;
var init_versions2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/skills/versions.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_uploads();
    init_path();
    Versions2 = class extends APIResource {
      /**
       * Create Skill Version
       *
       * @example
       * ```ts
       * const version = await client.beta.skills.versions.create(
       *   'skill_id',
       *   { files: [fs.createReadStream('path/to/file')] },
       * );
       * ```
       */
      create(skillID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/skills/${skillID}/versions?beta=true`, multipartFormRequestOptions({
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        }, this._client, false));
      }
      /**
       * Get Skill Version
       *
       * @example
       * ```ts
       * const version = await client.beta.skills.versions.retrieve(
       *   'version',
       *   { skill_id: 'skill_id' },
       * );
       * ```
       */
      retrieve(version, params, options) {
        const { skill_id, betas } = params;
        return this._client.get(path`/v1/skills/${skill_id}/versions/${version}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Skill Versions
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const versionListResponse of client.beta.skills.versions.list(
       *   'skill_id',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(skillID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/skills/${skillID}/versions?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete Skill Version
       *
       * @example
       * ```ts
       * const version = await client.beta.skills.versions.delete(
       *   'version',
       *   { skill_id: 'skill_id' },
       * );
       * ```
       */
      delete(version, params, options) {
        const { skill_id, betas } = params;
        return this._client.delete(path`/v1/skills/${skill_id}/versions/${version}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Download a skill version's content as a zip archive.
       *
       * @example
       * ```ts
       * const response = await client.beta.skills.versions.download(
       *   'version',
       *   { skill_id: 'skill_id' },
       * );
       *
       * const content = await response.blob();
       * console.log(content);
       * ```
       */
      download(version, params, options) {
        const { skill_id, betas } = params;
        return this._client.get(path`/v1/skills/${skill_id}/versions/${version}/content?beta=true`, {
          ...options,
          headers: buildHeaders([
            {
              "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString(),
              Accept: "application/binary"
            },
            options?.headers
          ]),
          __binaryResponse: true
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/skills/skills.mjs
var Skills;
var init_skills2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/skills/skills.mjs"() {
    init_resource();
    init_versions2();
    init_versions2();
    init_pagination();
    init_headers();
    init_uploads();
    init_path();
    Skills = class extends APIResource {
      constructor() {
        super(...arguments);
        this.versions = new Versions2(this._client);
      }
      /**
       * Create Skill
       *
       * @example
       * ```ts
       * const skill = await client.beta.skills.create({
       *   files: [fs.createReadStream('path/to/file')],
       * });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/skills?beta=true", multipartFormRequestOptions({
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        }, this._client, false));
      }
      /**
       * Get Skill
       *
       * @example
       * ```ts
       * const skill = await client.beta.skills.retrieve('skill_id');
       * ```
       */
      retrieve(skillID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/skills/${skillID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Skills
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const skillListResponse of client.beta.skills.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/skills?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete Skill
       *
       * @example
       * ```ts
       * const skill = await client.beta.skills.delete('skill_id');
       * ```
       */
      delete(skillID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/skills/${skillID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "skills-2025-10-02"].toString() },
            options?.headers
          ])
        });
      }
    };
    Skills.Versions = Versions2;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/tunnels/certificates.mjs
var Certificates;
var init_certificates = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/tunnels/certificates.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Certificates = class extends APIResource {
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Registers a public CA certificate on a tunnel. Anthropic verifies the gateway's
       * server certificate against this CA when it terminates the inner TLS session. A
       * tunnel holds at most two non-archived certificates.
       *
       * @example
       * ```ts
       * const betaTunnelCertificate =
       *   await client.beta.tunnels.certificates.create(
       *     'tunnel_id',
       *     { ca_certificate_pem: 'ca_certificate_pem' },
       *   );
       * ```
       */
      create(tunnelID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/tunnels/${tunnelID}/certificates?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Fetches a tunnel certificate by ID.
       *
       * @example
       * ```ts
       * const betaTunnelCertificate =
       *   await client.beta.tunnels.certificates.retrieve(
       *     'certificate_id',
       *     { tunnel_id: 'tunnel_id' },
       *   );
       * ```
       */
      retrieve(certificateID, params, options) {
        const { tunnel_id, betas } = params;
        return this._client.get(path`/v1/tunnels/${tunnel_id}/certificates/${certificateID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Lists the certificates registered on a tunnel. Archived certificates are
       * excluded unless include_archived is set.
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaTunnelCertificate of client.beta.tunnels.certificates.list(
       *   'tunnel_id',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(tunnelID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/tunnels/${tunnelID}/certificates?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Archives a tunnel certificate, removing it from the set Anthropic trusts for the
       * tunnel. The certificate record is retained. Archiving the last non-archived
       * certificate is permitted; the tunnel rejects MCP traffic until a new certificate
       * is added.
       *
       * @example
       * ```ts
       * const betaTunnelCertificate =
       *   await client.beta.tunnels.certificates.archive(
       *     'certificate_id',
       *     { tunnel_id: 'tunnel_id' },
       *   );
       * ```
       */
      archive(certificateID, params, options) {
        const { tunnel_id, betas } = params;
        return this._client.post(path`/v1/tunnels/${tunnel_id}/certificates/${certificateID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/tunnels/tunnels.mjs
var Tunnels;
var init_tunnels = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/tunnels/tunnels.mjs"() {
    init_resource();
    init_certificates();
    init_certificates();
    init_pagination();
    init_headers();
    init_path();
    Tunnels = class extends APIResource {
      constructor() {
        super(...arguments);
        this.certificates = new Certificates(this._client);
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Creates a tunnel. Creation allocates a fresh hostname and provisions the tunnel;
       * it is not idempotent. The new tunnel rejects MCP traffic until at least one CA
       * certificate is added.
       *
       * @example
       * ```ts
       * const betaTunnel = await client.beta.tunnels.create();
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/tunnels?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Fetches a tunnel by ID.
       *
       * @example
       * ```ts
       * const betaTunnel = await client.beta.tunnels.retrieve(
       *   'tunnel_id',
       * );
       * ```
       */
      retrieve(tunnelID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/tunnels/${tunnelID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Lists tunnels. Results are ordered by creation time, newest first; archived
       * tunnels are excluded unless include_archived is set.
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaTunnel of client.beta.tunnels.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/tunnels?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Archives a tunnel. Archival is irreversible: every non-archived certificate on
       * the tunnel is archived in the same operation, the hostname is retired and never
       * re-allocated, and the tunnel token is invalidated. Retrying against an
       * already-archived tunnel returns the existing record unchanged.
       *
       * @example
       * ```ts
       * const betaTunnel = await client.beta.tunnels.archive(
       *   'tunnel_id',
       * );
       * ```
       */
      archive(tunnelID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/tunnels/${tunnelID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Reveals a tunnel's connector token. The value is fetched live on each call;
       * Anthropic does not store it. Repeated calls return the same value until the
       * token is rotated. Exposed as POST so the token does not appear in intermediary
       * access logs.
       *
       * @example
       * ```ts
       * const betaTunnelToken =
       *   await client.beta.tunnels.revealToken('tunnel_id');
       * ```
       */
      revealToken(tunnelID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/tunnels/${tunnelID}/reveal_token?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * The Tunnels API is in research preview. It requires the
       * `anthropic-beta: mcp-tunnels-2026-06-22` header and may change without a
       * deprecation period. It supersedes the Admin API endpoints at
       * `/v1/organizations/tunnels`, which remain available during a migration window.
       *
       * Rotates a tunnel's connector token. Rotation invalidates the current token for
       * new connections and returns a fresh value; established connections are not
       * severed. A connector restarted after rotation must use the new value.
       *
       * @example
       * ```ts
       * const betaTunnelToken =
       *   await client.beta.tunnels.rotateToken('tunnel_id');
       * ```
       */
      rotateToken(tunnelID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/tunnels/${tunnelID}/rotate_token?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "mcp-tunnels-2026-06-22"].toString() },
            options?.headers
          ])
        });
      }
    };
    Tunnels.Certificates = Certificates;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/vaults/credentials.mjs
var Credentials;
var init_credentials2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/vaults/credentials.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Credentials = class extends APIResource {
      /**
       * Create Credential
       *
       * @example
       * ```ts
       * const betaManagedAgentsCredential =
       *   await client.beta.vaults.credentials.create(
       *     'vlt_011CZkZDLs7fYzm1hXNPeRjv',
       *     {
       *       auth: {
       *         token: 'bearer_exampletoken',
       *         mcp_server_url:
       *           'https://example-server.modelcontextprotocol.io/sse',
       *         type: 'static_bearer',
       *       },
       *     },
       *   );
       * ```
       */
      create(vaultID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/vaults/${vaultID}/credentials?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get Credential
       *
       * @example
       * ```ts
       * const betaManagedAgentsCredential =
       *   await client.beta.vaults.credentials.retrieve(
       *     'vcrd_011CZkZEMt8gZan2iYOQfSkw',
       *     { vault_id: 'vlt_011CZkZDLs7fYzm1hXNPeRjv' },
       *   );
       * ```
       */
      retrieve(credentialID, params, options) {
        const { vault_id, betas } = params;
        return this._client.get(path`/v1/vaults/${vault_id}/credentials/${credentialID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update Credential
       *
       * @example
       * ```ts
       * const betaManagedAgentsCredential =
       *   await client.beta.vaults.credentials.update(
       *     'vcrd_011CZkZEMt8gZan2iYOQfSkw',
       *     { vault_id: 'vlt_011CZkZDLs7fYzm1hXNPeRjv' },
       *   );
       * ```
       */
      update(credentialID, params, options) {
        const { vault_id, betas, ...body } = params;
        return this._client.post(path`/v1/vaults/${vault_id}/credentials/${credentialID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Credentials
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsCredential of client.beta.vaults.credentials.list(
       *   'vlt_011CZkZDLs7fYzm1hXNPeRjv',
       * )) {
       *   // ...
       * }
       * ```
       */
      list(vaultID, params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList(path`/v1/vaults/${vaultID}/credentials?beta=true`, PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete Credential
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeletedCredential =
       *   await client.beta.vaults.credentials.delete(
       *     'vcrd_011CZkZEMt8gZan2iYOQfSkw',
       *     { vault_id: 'vlt_011CZkZDLs7fYzm1hXNPeRjv' },
       *   );
       * ```
       */
      delete(credentialID, params, options) {
        const { vault_id, betas } = params;
        return this._client.delete(path`/v1/vaults/${vault_id}/credentials/${credentialID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive Credential
       *
       * @example
       * ```ts
       * const betaManagedAgentsCredential =
       *   await client.beta.vaults.credentials.archive(
       *     'vcrd_011CZkZEMt8gZan2iYOQfSkw',
       *     { vault_id: 'vlt_011CZkZDLs7fYzm1hXNPeRjv' },
       *   );
       * ```
       */
      archive(credentialID, params, options) {
        const { vault_id, betas } = params;
        return this._client.post(path`/v1/vaults/${vault_id}/credentials/${credentialID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Validate Credential
       *
       * @example
       * ```ts
       * const betaManagedAgentsCredentialValidation =
       *   await client.beta.vaults.credentials.mcpOAuthValidate(
       *     'vcrd_011CZkZEMt8gZan2iYOQfSkw',
       *     { vault_id: 'vlt_011CZkZDLs7fYzm1hXNPeRjv' },
       *   );
       * ```
       */
      mcpOAuthValidate(credentialID, params, options) {
        const { vault_id, betas } = params;
        return this._client.post(path`/v1/vaults/${vault_id}/credentials/${credentialID}/mcp_oauth_validate?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/vaults/vaults.mjs
var Vaults;
var init_vaults = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/vaults/vaults.mjs"() {
    init_resource();
    init_credentials2();
    init_credentials2();
    init_pagination();
    init_headers();
    init_path();
    Vaults = class extends APIResource {
      constructor() {
        super(...arguments);
        this.credentials = new Credentials(this._client);
      }
      /**
       * Create Vault
       *
       * @example
       * ```ts
       * const betaManagedAgentsVault =
       *   await client.beta.vaults.create({
       *     display_name: 'Example vault',
       *   });
       * ```
       */
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/vaults?beta=true", {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Get Vault
       *
       * @example
       * ```ts
       * const betaManagedAgentsVault =
       *   await client.beta.vaults.retrieve(
       *     'vlt_011CZkZDLs7fYzm1hXNPeRjv',
       *   );
       * ```
       */
      retrieve(vaultID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/vaults/${vaultID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Update Vault
       *
       * @example
       * ```ts
       * const betaManagedAgentsVault =
       *   await client.beta.vaults.update(
       *     'vlt_011CZkZDLs7fYzm1hXNPeRjv',
       *   );
       * ```
       */
      update(vaultID, params, options) {
        const { betas, ...body } = params;
        return this._client.post(path`/v1/vaults/${vaultID}?beta=true`, {
          body,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * List Vaults
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const betaManagedAgentsVault of client.beta.vaults.list()) {
       *   // ...
       * }
       * ```
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/vaults?beta=true", PageCursor, {
          query,
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Delete Vault
       *
       * @example
       * ```ts
       * const betaManagedAgentsDeletedVault =
       *   await client.beta.vaults.delete(
       *     'vlt_011CZkZDLs7fYzm1hXNPeRjv',
       *   );
       * ```
       */
      delete(vaultID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.delete(path`/v1/vaults/${vaultID}?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
      /**
       * Archive Vault
       *
       * @example
       * ```ts
       * const betaManagedAgentsVault =
       *   await client.beta.vaults.archive(
       *     'vlt_011CZkZDLs7fYzm1hXNPeRjv',
       *   );
       * ```
       */
      archive(vaultID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.post(path`/v1/vaults/${vaultID}/archive?beta=true`, {
          ...options,
          headers: buildHeaders([
            { "anthropic-beta": [...betas ?? [], "managed-agents-2026-04-01"].toString() },
            options?.headers
          ])
        });
      }
    };
    Vaults.Credentials = Credentials;
  }
});

// node_modules/@anthropic-ai/sdk/resources/beta/beta.mjs
var Beta;
var init_beta = __esm({
  "node_modules/@anthropic-ai/sdk/resources/beta/beta.mjs"() {
    init_resource();
    init_deployment_runs();
    init_deployment_runs();
    init_deployments();
    init_deployments();
    init_dreams();
    init_dreams();
    init_files();
    init_files();
    init_models();
    init_models();
    init_user_profiles();
    init_user_profiles();
    init_webhooks();
    init_webhooks();
    init_agents();
    init_agents();
    init_environments();
    init_environments();
    init_memory_stores();
    init_memory_stores();
    init_messages();
    init_messages();
    init_sessions();
    init_sessions();
    init_skills2();
    init_skills2();
    init_tunnels();
    init_tunnels();
    init_vaults();
    init_vaults();
    Beta = class extends APIResource {
      constructor() {
        super(...arguments);
        this.models = new Models(this._client);
        this.messages = new Messages(this._client);
        this.agents = new Agents(this._client);
        this.environments = new Environments(this._client);
        this.sessions = new Sessions(this._client);
        this.deployments = new Deployments(this._client);
        this.deploymentRuns = new DeploymentRuns(this._client);
        this.vaults = new Vaults(this._client);
        this.memoryStores = new MemoryStores(this._client);
        this.files = new Files(this._client);
        this.skills = new Skills(this._client);
        this.webhooks = new Webhooks(this._client);
        this.userProfiles = new UserProfiles(this._client);
        this.dreams = new Dreams(this._client);
        this.tunnels = new Tunnels(this._client);
      }
    };
    Beta.Models = Models;
    Beta.Messages = Messages;
    Beta.Agents = Agents;
    Beta.Environments = Environments;
    Beta.Sessions = Sessions;
    Beta.Deployments = Deployments;
    Beta.DeploymentRuns = DeploymentRuns;
    Beta.Vaults = Vaults;
    Beta.MemoryStores = MemoryStores;
    Beta.Files = Files;
    Beta.Skills = Skills;
    Beta.Webhooks = Webhooks;
    Beta.UserProfiles = UserProfiles;
    Beta.Dreams = Dreams;
    Beta.Tunnels = Tunnels;
  }
});

// node_modules/@anthropic-ai/sdk/resources/completions.mjs
var Completions;
var init_completions = __esm({
  "node_modules/@anthropic-ai/sdk/resources/completions.mjs"() {
    init_resource();
    init_headers();
    Completions = class extends APIResource {
      create(params, options) {
        const { betas, ...body } = params;
        return this._client.post("/v1/complete", {
          body,
          timeout: this._client._options.timeout ?? 6e5,
          ...options,
          headers: buildHeaders([
            { ...betas?.toString() != null ? { "anthropic-beta": betas?.toString() } : void 0 },
            options?.headers
          ]),
          stream: params.stream ?? false
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/lib/parser.mjs
function getOutputFormat2(params) {
  return params?.output_config?.format;
}
function maybeParseMessage(message, params, opts) {
  const outputFormat = getOutputFormat2(params);
  if (!params || !("parse" in (outputFormat ?? {}))) {
    return {
      ...message,
      content: message.content.map((block) => {
        if (block.type === "text") {
          const parsedBlock = Object.defineProperty({ ...block }, "parsed_output", {
            value: null,
            enumerable: false
          });
          return parsedBlock;
        }
        return block;
      }),
      parsed_output: null
    };
  }
  return parseMessage(message, params, opts);
}
function parseMessage(message, params, opts) {
  let firstParsedOutput = null;
  const content = message.content.map((block) => {
    if (block.type === "text") {
      const parsedOutput = parseOutputFormat(params, block.text);
      if (firstParsedOutput === null) {
        firstParsedOutput = parsedOutput;
      }
      const parsedBlock = Object.defineProperty({ ...block }, "parsed_output", {
        value: parsedOutput,
        enumerable: false
      });
      return parsedBlock;
    }
    return block;
  });
  return {
    ...message,
    content,
    parsed_output: firstParsedOutput
  };
}
function parseOutputFormat(params, content) {
  const outputFormat = getOutputFormat2(params);
  if (outputFormat?.type !== "json_schema") {
    return null;
  }
  try {
    if ("parse" in outputFormat) {
      return outputFormat.parse(content);
    }
    return JSON.parse(content);
  } catch (error) {
    throw new AnthropicError(`Failed to parse structured output: ${error}`);
  }
}
var init_parser2 = __esm({
  "node_modules/@anthropic-ai/sdk/lib/parser.mjs"() {
    init_error();
  }
});

// node_modules/@anthropic-ai/sdk/lib/MessageStream.mjs
function tracksToolInput2(content) {
  return content.type === "tool_use" || content.type === "server_tool_use";
}
function checkNever2(x) {
}
var _MessageStream_instances, _MessageStream_currentMessageSnapshot, _MessageStream_params, _MessageStream_connectedPromise, _MessageStream_resolveConnectedPromise, _MessageStream_rejectConnectedPromise, _MessageStream_endPromise, _MessageStream_resolveEndPromise, _MessageStream_rejectEndPromise, _MessageStream_listeners, _MessageStream_ended, _MessageStream_errored, _MessageStream_aborted, _MessageStream_catchingPromiseCreated, _MessageStream_response, _MessageStream_request_id, _MessageStream_logger, _MessageStream_getFinalMessage, _MessageStream_getFinalText, _MessageStream_handleError, _MessageStream_beginRequest, _MessageStream_addStreamEvent, _MessageStream_endRequest, _MessageStream_accumulateMessage, MessageStream;
var init_MessageStream = __esm({
  "node_modules/@anthropic-ai/sdk/lib/MessageStream.mjs"() {
    init_tslib();
    init_stainless_helper_header();
    init_errors();
    init_error2();
    init_streaming2();
    init_parser2();
    init_message_stream_utils();
    MessageStream = class _MessageStream {
      constructor(params, opts) {
        _MessageStream_instances.add(this);
        this.messages = [];
        this.receivedMessages = [];
        _MessageStream_currentMessageSnapshot.set(this, void 0);
        _MessageStream_params.set(this, null);
        this.controller = new AbortController();
        _MessageStream_connectedPromise.set(this, void 0);
        _MessageStream_resolveConnectedPromise.set(this, () => {
        });
        _MessageStream_rejectConnectedPromise.set(this, () => {
        });
        _MessageStream_endPromise.set(this, void 0);
        _MessageStream_resolveEndPromise.set(this, () => {
        });
        _MessageStream_rejectEndPromise.set(this, () => {
        });
        _MessageStream_listeners.set(this, {});
        _MessageStream_ended.set(this, false);
        _MessageStream_errored.set(this, false);
        _MessageStream_aborted.set(this, false);
        _MessageStream_catchingPromiseCreated.set(this, false);
        _MessageStream_response.set(this, void 0);
        _MessageStream_request_id.set(this, void 0);
        _MessageStream_logger.set(this, void 0);
        _MessageStream_handleError.set(this, (error) => {
          __classPrivateFieldSet(this, _MessageStream_errored, true, "f");
          if (isAbortError(error)) {
            error = new APIUserAbortError();
          }
          if (error instanceof APIUserAbortError) {
            __classPrivateFieldSet(this, _MessageStream_aborted, true, "f");
            return this._emit("abort", error);
          }
          if (error instanceof AnthropicError) {
            return this._emit("error", error);
          }
          if (error instanceof Error) {
            const anthropicError = new AnthropicError(error.message);
            anthropicError.cause = error;
            return this._emit("error", anthropicError);
          }
          return this._emit("error", new AnthropicError(String(error)));
        });
        __classPrivateFieldSet(this, _MessageStream_connectedPromise, new Promise((resolve4, reject) => {
          __classPrivateFieldSet(this, _MessageStream_resolveConnectedPromise, resolve4, "f");
          __classPrivateFieldSet(this, _MessageStream_rejectConnectedPromise, reject, "f");
        }), "f");
        __classPrivateFieldSet(this, _MessageStream_endPromise, new Promise((resolve4, reject) => {
          __classPrivateFieldSet(this, _MessageStream_resolveEndPromise, resolve4, "f");
          __classPrivateFieldSet(this, _MessageStream_rejectEndPromise, reject, "f");
        }), "f");
        __classPrivateFieldGet(this, _MessageStream_connectedPromise, "f").catch(() => {
        });
        __classPrivateFieldGet(this, _MessageStream_endPromise, "f").catch(() => {
        });
        __classPrivateFieldSet(this, _MessageStream_params, params, "f");
        __classPrivateFieldSet(this, _MessageStream_logger, opts?.logger ?? console, "f");
      }
      get response() {
        return __classPrivateFieldGet(this, _MessageStream_response, "f");
      }
      get request_id() {
        return __classPrivateFieldGet(this, _MessageStream_request_id, "f");
      }
      /**
       * Returns the `MessageStream` data, the raw `Response` instance and the ID of the request,
       * returned vie the `request-id` header which is useful for debugging requests and resporting
       * issues to Anthropic.
       *
       * This is the same as the `APIPromise.withResponse()` method.
       *
       * This method will raise an error if you created the stream using `MessageStream.fromReadableStream`
       * as no `Response` is available.
       */
      async withResponse() {
        __classPrivateFieldSet(this, _MessageStream_catchingPromiseCreated, true, "f");
        const response = await __classPrivateFieldGet(this, _MessageStream_connectedPromise, "f");
        if (!response) {
          throw new Error("Could not resolve a `Response` object");
        }
        return {
          data: this,
          response,
          request_id: response.headers.get("request-id")
        };
      }
      /**
       * Intended for use on the frontend, consuming a stream produced with
       * `.toReadableStream()` on the backend.
       *
       * Note that messages sent to the model do not appear in `.on('message')`
       * in this context.
       */
      static fromReadableStream(stream) {
        const runner = new _MessageStream(null);
        runner._run(() => runner._fromReadableStream(stream));
        return runner;
      }
      static createMessage(messages, params, options, { logger } = {}) {
        const runner = new _MessageStream(params, { logger });
        for (const message of params.messages) {
          runner._addMessageParam(message);
        }
        __classPrivateFieldSet(runner, _MessageStream_params, { ...params, stream: true }, "f");
        runner._run(() => runner._createMessage(messages, { ...params, stream: true }, { ...options, headers: { ...options?.headers, [STAINLESS_HELPER_METHOD_HEADER]: "stream" } }));
        return runner;
      }
      _run(executor) {
        executor().then(() => {
          this._emitFinal();
          this._emit("end");
        }, __classPrivateFieldGet(this, _MessageStream_handleError, "f"));
      }
      _addMessageParam(message) {
        this.messages.push(message);
      }
      _addMessage(message, emit = true) {
        this.receivedMessages.push(message);
        if (emit) {
          this._emit("message", message);
        }
      }
      async _createMessage(messages, params, options) {
        const signal = options?.signal;
        let abortHandler;
        if (signal) {
          if (signal.aborted)
            this.controller.abort();
          abortHandler = this.controller.abort.bind(this.controller);
          signal.addEventListener("abort", abortHandler);
        }
        try {
          __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_beginRequest).call(this);
          const { response, data: stream } = await messages.create({ ...params, stream: true }, { ...options, signal: this.controller.signal }).withResponse();
          this._connected(response);
          for await (const event of stream) {
            __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_addStreamEvent).call(this, event);
          }
          if (stream.controller.signal?.aborted) {
            throw new APIUserAbortError();
          }
          __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_endRequest).call(this);
        } finally {
          if (signal && abortHandler) {
            signal.removeEventListener("abort", abortHandler);
          }
        }
      }
      _connected(response) {
        if (this.ended)
          return;
        __classPrivateFieldSet(this, _MessageStream_response, response, "f");
        __classPrivateFieldSet(this, _MessageStream_request_id, response?.headers.get("request-id"), "f");
        __classPrivateFieldGet(this, _MessageStream_resolveConnectedPromise, "f").call(this, response);
        this._emit("connect");
      }
      get ended() {
        return __classPrivateFieldGet(this, _MessageStream_ended, "f");
      }
      get errored() {
        return __classPrivateFieldGet(this, _MessageStream_errored, "f");
      }
      get aborted() {
        return __classPrivateFieldGet(this, _MessageStream_aborted, "f");
      }
      abort() {
        this.controller.abort();
      }
      /**
       * Adds the listener function to the end of the listeners array for the event.
       * No checks are made to see if the listener has already been added. Multiple calls passing
       * the same combination of event and listener will result in the listener being added, and
       * called, multiple times.
       * @returns this MessageStream, so that calls can be chained
       */
      on(event, listener) {
        const listeners = __classPrivateFieldGet(this, _MessageStream_listeners, "f")[event] || (__classPrivateFieldGet(this, _MessageStream_listeners, "f")[event] = []);
        listeners.push({ listener });
        return this;
      }
      /**
       * Removes the specified listener from the listener array for the event.
       * off() will remove, at most, one instance of a listener from the listener array. If any single
       * listener has been added multiple times to the listener array for the specified event, then
       * off() must be called multiple times to remove each instance.
       * @returns this MessageStream, so that calls can be chained
       */
      off(event, listener) {
        const listeners = __classPrivateFieldGet(this, _MessageStream_listeners, "f")[event];
        if (!listeners)
          return this;
        const index = listeners.findIndex((l) => l.listener === listener);
        if (index >= 0)
          listeners.splice(index, 1);
        return this;
      }
      /**
       * Adds a one-time listener function for the event. The next time the event is triggered,
       * this listener is removed and then invoked.
       * @returns this MessageStream, so that calls can be chained
       */
      once(event, listener) {
        const listeners = __classPrivateFieldGet(this, _MessageStream_listeners, "f")[event] || (__classPrivateFieldGet(this, _MessageStream_listeners, "f")[event] = []);
        listeners.push({ listener, once: true });
        return this;
      }
      /**
       * This is similar to `.once()`, but returns a Promise that resolves the next time
       * the event is triggered, instead of calling a listener callback.
       * @returns a Promise that resolves the next time given event is triggered,
       * or rejects if an error is emitted.  (If you request the 'error' event,
       * returns a promise that resolves with the error).
       *
       * Example:
       *
       *   const message = await stream.emitted('message') // rejects if the stream errors
       */
      emitted(event) {
        return new Promise((resolve4, reject) => {
          __classPrivateFieldSet(this, _MessageStream_catchingPromiseCreated, true, "f");
          if (event !== "error")
            this.once("error", reject);
          this.once(event, resolve4);
        });
      }
      async done() {
        __classPrivateFieldSet(this, _MessageStream_catchingPromiseCreated, true, "f");
        await __classPrivateFieldGet(this, _MessageStream_endPromise, "f");
      }
      get currentMessage() {
        return __classPrivateFieldGet(this, _MessageStream_currentMessageSnapshot, "f");
      }
      /**
       * @returns a promise that resolves with the the final assistant Message response,
       * or rejects if an error occurred or the stream ended prematurely without producing a Message.
       * If structured outputs were used, this will be a ParsedMessage with a `parsed_output` field.
       */
      async finalMessage() {
        await this.done();
        return __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_getFinalMessage).call(this);
      }
      /**
       * @returns a promise that resolves with the the final assistant Message's text response, concatenated
       * together if there are more than one text blocks.
       * Rejects if an error occurred or the stream ended prematurely without producing a Message.
       */
      async finalText() {
        await this.done();
        return __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_getFinalText).call(this);
      }
      _emit(event, ...args) {
        if (__classPrivateFieldGet(this, _MessageStream_ended, "f"))
          return;
        if (event === "end") {
          __classPrivateFieldSet(this, _MessageStream_ended, true, "f");
          __classPrivateFieldGet(this, _MessageStream_resolveEndPromise, "f").call(this);
        }
        const listeners = __classPrivateFieldGet(this, _MessageStream_listeners, "f")[event];
        if (listeners) {
          __classPrivateFieldGet(this, _MessageStream_listeners, "f")[event] = listeners.filter((l) => !l.once);
          listeners.forEach(({ listener }) => listener(...args));
        }
        if (event === "abort") {
          const error = args[0];
          if (!__classPrivateFieldGet(this, _MessageStream_catchingPromiseCreated, "f") && !listeners?.length) {
            Promise.reject(error);
          }
          __classPrivateFieldGet(this, _MessageStream_rejectConnectedPromise, "f").call(this, error);
          __classPrivateFieldGet(this, _MessageStream_rejectEndPromise, "f").call(this, error);
          this._emit("end");
          return;
        }
        if (event === "error") {
          const error = args[0];
          if (!__classPrivateFieldGet(this, _MessageStream_catchingPromiseCreated, "f") && !listeners?.length) {
            Promise.reject(error);
          }
          __classPrivateFieldGet(this, _MessageStream_rejectConnectedPromise, "f").call(this, error);
          __classPrivateFieldGet(this, _MessageStream_rejectEndPromise, "f").call(this, error);
          this._emit("end");
        }
      }
      _emitFinal() {
        const finalMessage = this.receivedMessages.at(-1);
        if (finalMessage) {
          this._emit("finalMessage", __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_getFinalMessage).call(this));
        }
      }
      async _fromReadableStream(readableStream, options) {
        const signal = options?.signal;
        let abortHandler;
        if (signal) {
          if (signal.aborted)
            this.controller.abort();
          abortHandler = this.controller.abort.bind(this.controller);
          signal.addEventListener("abort", abortHandler);
        }
        try {
          __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_beginRequest).call(this);
          this._connected(null);
          const stream = Stream.fromReadableStream(readableStream, this.controller);
          for await (const event of stream) {
            __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_addStreamEvent).call(this, event);
          }
          if (stream.controller.signal?.aborted) {
            throw new APIUserAbortError();
          }
          __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_endRequest).call(this);
        } finally {
          if (signal && abortHandler) {
            signal.removeEventListener("abort", abortHandler);
          }
        }
      }
      [(_MessageStream_currentMessageSnapshot = /* @__PURE__ */ new WeakMap(), _MessageStream_params = /* @__PURE__ */ new WeakMap(), _MessageStream_connectedPromise = /* @__PURE__ */ new WeakMap(), _MessageStream_resolveConnectedPromise = /* @__PURE__ */ new WeakMap(), _MessageStream_rejectConnectedPromise = /* @__PURE__ */ new WeakMap(), _MessageStream_endPromise = /* @__PURE__ */ new WeakMap(), _MessageStream_resolveEndPromise = /* @__PURE__ */ new WeakMap(), _MessageStream_rejectEndPromise = /* @__PURE__ */ new WeakMap(), _MessageStream_listeners = /* @__PURE__ */ new WeakMap(), _MessageStream_ended = /* @__PURE__ */ new WeakMap(), _MessageStream_errored = /* @__PURE__ */ new WeakMap(), _MessageStream_aborted = /* @__PURE__ */ new WeakMap(), _MessageStream_catchingPromiseCreated = /* @__PURE__ */ new WeakMap(), _MessageStream_response = /* @__PURE__ */ new WeakMap(), _MessageStream_request_id = /* @__PURE__ */ new WeakMap(), _MessageStream_logger = /* @__PURE__ */ new WeakMap(), _MessageStream_handleError = /* @__PURE__ */ new WeakMap(), _MessageStream_instances = /* @__PURE__ */ new WeakSet(), _MessageStream_getFinalMessage = function _MessageStream_getFinalMessage2() {
        if (this.receivedMessages.length === 0) {
          throw new AnthropicError("stream ended without producing a Message with role=assistant");
        }
        return this.receivedMessages.at(-1);
      }, _MessageStream_getFinalText = function _MessageStream_getFinalText2() {
        if (this.receivedMessages.length === 0) {
          throw new AnthropicError("stream ended without producing a Message with role=assistant");
        }
        const textBlocks = this.receivedMessages.at(-1).content.filter((block) => block.type === "text").map((block) => block.text);
        if (textBlocks.length === 0) {
          throw new AnthropicError("stream ended without producing a content block with type=text");
        }
        return textBlocks.join(" ");
      }, _MessageStream_beginRequest = function _MessageStream_beginRequest2() {
        if (this.ended)
          return;
        __classPrivateFieldSet(this, _MessageStream_currentMessageSnapshot, void 0, "f");
      }, _MessageStream_addStreamEvent = function _MessageStream_addStreamEvent2(event) {
        if (this.ended)
          return;
        const messageSnapshot = __classPrivateFieldGet(this, _MessageStream_instances, "m", _MessageStream_accumulateMessage).call(this, event);
        this._emit("streamEvent", event, messageSnapshot);
        switch (event.type) {
          case "content_block_delta": {
            const content = messageSnapshot.content.at(-1);
            switch (event.delta.type) {
              case "text_delta": {
                if (content.type === "text") {
                  this._emit("text", event.delta.text, content.text || "");
                }
                break;
              }
              case "citations_delta": {
                if (content.type === "text") {
                  this._emit("citation", event.delta.citation, content.citations ?? []);
                }
                break;
              }
              case "input_json_delta": {
                if (tracksToolInput2(content) && __classPrivateFieldGet(this, _MessageStream_listeners, "f").inputJson?.length) {
                  this._emit("inputJson", event.delta.partial_json, content.input);
                }
                break;
              }
              case "thinking_delta": {
                if (content.type === "thinking") {
                  this._emit("thinking", event.delta.thinking, content.thinking);
                }
                break;
              }
              case "signature_delta": {
                if (content.type === "thinking") {
                  this._emit("signature", content.signature);
                }
                break;
              }
              default:
                checkNever2(event.delta);
            }
            break;
          }
          case "message_stop": {
            this._addMessageParam(messageSnapshot);
            this._addMessage(maybeParseMessage(messageSnapshot, __classPrivateFieldGet(this, _MessageStream_params, "f"), { logger: __classPrivateFieldGet(this, _MessageStream_logger, "f") }), true);
            break;
          }
          case "content_block_stop": {
            this._emit("contentBlock", messageSnapshot.content.at(-1));
            break;
          }
          case "message_start": {
            __classPrivateFieldSet(this, _MessageStream_currentMessageSnapshot, messageSnapshot, "f");
            break;
          }
          case "content_block_start":
          case "message_delta":
            break;
        }
      }, _MessageStream_endRequest = function _MessageStream_endRequest2() {
        if (this.ended) {
          throw new AnthropicError(`stream has ended, this shouldn't happen`);
        }
        const snapshot = __classPrivateFieldGet(this, _MessageStream_currentMessageSnapshot, "f");
        if (!snapshot) {
          throw new AnthropicError(`request ended without sending any chunks`);
        }
        __classPrivateFieldSet(this, _MessageStream_currentMessageSnapshot, void 0, "f");
        return maybeParseMessage(snapshot, __classPrivateFieldGet(this, _MessageStream_params, "f"), { logger: __classPrivateFieldGet(this, _MessageStream_logger, "f") });
      }, _MessageStream_accumulateMessage = function _MessageStream_accumulateMessage2(event) {
        let snapshot = __classPrivateFieldGet(this, _MessageStream_currentMessageSnapshot, "f");
        if (event.type === "message_start") {
          if (snapshot) {
            throw new AnthropicError(`Unexpected event order, got ${event.type} before receiving "message_stop"`);
          }
          return event.message;
        }
        if (!snapshot) {
          throw new AnthropicError(`Unexpected event order, got ${event.type} before "message_start"`);
        }
        switch (event.type) {
          case "message_stop":
            return snapshot;
          case "message_delta":
            snapshot.stop_reason = event.delta.stop_reason;
            snapshot.stop_sequence = event.delta.stop_sequence;
            if (event.delta.stop_details != null) {
              snapshot.stop_details = event.delta.stop_details;
            }
            snapshot.usage.output_tokens = event.usage.output_tokens;
            if (event.usage.input_tokens != null) {
              snapshot.usage.input_tokens = event.usage.input_tokens;
            }
            if (event.usage.cache_creation_input_tokens != null) {
              snapshot.usage.cache_creation_input_tokens = event.usage.cache_creation_input_tokens;
            }
            if (event.usage.cache_read_input_tokens != null) {
              snapshot.usage.cache_read_input_tokens = event.usage.cache_read_input_tokens;
            }
            if (event.usage.server_tool_use != null) {
              snapshot.usage.server_tool_use = event.usage.server_tool_use;
            }
            return snapshot;
          case "content_block_start":
            snapshot.content.push({ ...event.content_block });
            return snapshot;
          case "content_block_delta": {
            const snapshotContent = snapshot.content.at(event.index);
            switch (event.delta.type) {
              case "text_delta": {
                if (snapshotContent?.type === "text") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    text: (snapshotContent.text || "") + event.delta.text
                  };
                }
                break;
              }
              case "citations_delta": {
                if (snapshotContent?.type === "text") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    citations: [...snapshotContent.citations ?? [], event.delta.citation]
                  };
                }
                break;
              }
              case "input_json_delta": {
                if (snapshotContent && tracksToolInput2(snapshotContent)) {
                  const jsonBuf = (snapshotContent[JSON_BUF_PROPERTY] || "") + event.delta.partial_json;
                  snapshot.content[event.index] = withLazyInput(snapshotContent, jsonBuf);
                }
                break;
              }
              case "thinking_delta": {
                if (snapshotContent?.type === "thinking") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    thinking: snapshotContent.thinking + event.delta.thinking
                  };
                }
                break;
              }
              case "signature_delta": {
                if (snapshotContent?.type === "thinking") {
                  snapshot.content[event.index] = {
                    ...snapshotContent,
                    signature: event.delta.signature
                  };
                }
                break;
              }
              default:
                checkNever2(event.delta);
            }
            return snapshot;
          }
          case "content_block_stop": {
            const snapshotContent = snapshot.content.at(event.index);
            if (snapshotContent && tracksToolInput2(snapshotContent) && JSON_BUF_PROPERTY in snapshotContent) {
              Object.defineProperty(snapshotContent, "input", {
                value: snapshotContent.input,
                enumerable: true,
                configurable: true,
                writable: true
              });
            }
            return snapshot;
          }
        }
      }, Symbol.asyncIterator)]() {
        const pushQueue = [];
        const readQueue = [];
        let done = false;
        this.on("streamEvent", (event) => {
          const reader = readQueue.shift();
          if (reader) {
            reader.resolve(event);
          } else {
            pushQueue.push(event);
          }
        });
        this.on("end", () => {
          done = true;
          for (const reader of readQueue) {
            reader.resolve(void 0);
          }
          readQueue.length = 0;
        });
        this.on("abort", (err) => {
          done = true;
          for (const reader of readQueue) {
            reader.reject(err);
          }
          readQueue.length = 0;
        });
        this.on("error", (err) => {
          done = true;
          for (const reader of readQueue) {
            reader.reject(err);
          }
          readQueue.length = 0;
        });
        return {
          next: async () => {
            if (!pushQueue.length) {
              if (done) {
                return { value: void 0, done: true };
              }
              return new Promise((resolve4, reject) => readQueue.push({ resolve: resolve4, reject })).then((chunk2) => chunk2 ? { value: chunk2, done: false } : { value: void 0, done: true });
            }
            const chunk = pushQueue.shift();
            return { value: chunk, done: false };
          },
          return: async () => {
            this.abort();
            return { value: void 0, done: true };
          }
        };
      }
      toReadableStream() {
        const stream = new Stream(this[Symbol.asyncIterator].bind(this), this.controller);
        return stream.toReadableStream();
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/messages/batches.mjs
var Batches2;
var init_batches2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/messages/batches.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_jsonl();
    init_error2();
    init_path();
    Batches2 = class extends APIResource {
      /**
       * Send a batch of Message creation requests.
       *
       * The Message Batches API can be used to process multiple Messages API requests at
       * once. Once a Message Batch is created, it begins processing immediately. Batches
       * can take up to 24 hours to complete.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const messageBatch = await client.messages.batches.create({
       *   requests: [
       *     {
       *       custom_id: 'my-custom-id-1',
       *       params: {
       *         max_tokens: 1024,
       *         messages: [
       *           { content: 'Hello, world', role: 'user' },
       *         ],
       *         model: 'claude-opus-4-6',
       *       },
       *     },
       *   ],
       * });
       * ```
       */
      create(params, options) {
        const { user_profile_id, ...body } = params;
        return this._client.post("/v1/messages/batches", {
          body,
          ...options,
          headers: buildHeaders([
            { ...user_profile_id != null ? { "anthropic-user-profile-id": user_profile_id } : void 0 },
            options?.headers
          ])
        });
      }
      /**
       * This endpoint is idempotent and can be used to poll for Message Batch
       * completion. To access the results of a Message Batch, make a request to the
       * `results_url` field in the response.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const messageBatch = await client.messages.batches.retrieve(
       *   'message_batch_id',
       * );
       * ```
       */
      retrieve(messageBatchID, options) {
        return this._client.get(path`/v1/messages/batches/${messageBatchID}`, options);
      }
      /**
       * List all Message Batches within a Workspace. Most recently created batches are
       * returned first.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * // Automatically fetches more pages as needed.
       * for await (const messageBatch of client.messages.batches.list()) {
       *   // ...
       * }
       * ```
       */
      list(query = {}, options) {
        return this._client.getAPIList("/v1/messages/batches", Page, { query, ...options });
      }
      /**
       * Delete a Message Batch.
       *
       * Message Batches can only be deleted once they've finished processing. If you'd
       * like to delete an in-progress batch, you must first cancel it.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const deletedMessageBatch =
       *   await client.messages.batches.delete('message_batch_id');
       * ```
       */
      delete(messageBatchID, options) {
        return this._client.delete(path`/v1/messages/batches/${messageBatchID}`, options);
      }
      /**
       * Batches may be canceled any time before processing ends. Once cancellation is
       * initiated, the batch enters a `canceling` state, at which time the system may
       * complete any in-progress, non-interruptible requests before finalizing
       * cancellation.
       *
       * The number of canceled requests is specified in `request_counts`. To determine
       * which requests were canceled, check the individual results within the batch.
       * Note that cancellation may not result in any canceled requests if they were
       * non-interruptible.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const messageBatch = await client.messages.batches.cancel(
       *   'message_batch_id',
       * );
       * ```
       */
      cancel(messageBatchID, options) {
        return this._client.post(path`/v1/messages/batches/${messageBatchID}/cancel`, options);
      }
      /**
       * Streams the results of a Message Batch as a `.jsonl` file.
       *
       * Each line in the file is a JSON object containing the result of a single request
       * in the Message Batch. Results are not guaranteed to be in the same order as
       * requests. Use the `custom_id` field to match results to requests.
       *
       * Learn more about the Message Batches API in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/batch-processing)
       *
       * @example
       * ```ts
       * const messageBatchIndividualResponse =
       *   await client.messages.batches.results('message_batch_id');
       * ```
       */
      async results(messageBatchID, options) {
        const batch = await this.retrieve(messageBatchID);
        if (!batch.results_url) {
          throw new AnthropicError(`No batch \`results_url\`; Has it finished processing? ${batch.processing_status} - ${batch.id}`);
        }
        return this._client.get(batch.results_url, {
          ...options,
          headers: buildHeaders([{ Accept: "application/binary" }, options?.headers]),
          stream: true,
          __binaryResponse: true
        })._thenUnwrap((_, props) => JSONLDecoder.fromResponse(props.response, props.controller));
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/messages/messages.mjs
var Messages2, DEPRECATED_MODELS2, MODELS_TO_WARN_WITH_THINKING_ENABLED2;
var init_messages2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/messages/messages.mjs"() {
    init_resource();
    init_headers();
    init_stainless_helper_header();
    init_MessageStream();
    init_parser2();
    init_batches2();
    init_batches2();
    init_constants();
    Messages2 = class extends APIResource {
      constructor() {
        super(...arguments);
        this.batches = new Batches2(this._client);
      }
      create(params, options) {
        const { user_profile_id, ...body } = params;
        if (body.model in DEPRECATED_MODELS2) {
          console.warn(`The model '${body.model}' is deprecated and will reach end-of-life on ${DEPRECATED_MODELS2[body.model]}
Please migrate to a newer model. Visit https://docs.anthropic.com/en/docs/resources/model-deprecations for more information.`);
        }
        if (MODELS_TO_WARN_WITH_THINKING_ENABLED2.includes(body.model) && body.thinking && body.thinking.type === "enabled") {
          console.warn(`Using Claude with ${body.model} and 'thinking.type=enabled' is deprecated. Use 'thinking.type=adaptive' instead which results in better model performance in our testing: https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking`);
        }
        let timeout = this._client._options.timeout;
        if (!body.stream && timeout == null) {
          const maxNonstreamingTokens = MODEL_NONSTREAMING_TOKENS[body.model] ?? void 0;
          timeout = this._client.calculateNonstreamingTimeout(body.max_tokens, maxNonstreamingTokens);
        }
        const helperHeader2 = stainlessHelperHeader(body.tools, body.messages);
        return this._client.post("/v1/messages", {
          body,
          timeout: timeout ?? 6e5,
          ...options,
          headers: buildHeaders([
            { ...user_profile_id != null ? { "anthropic-user-profile-id": user_profile_id } : void 0 },
            helperHeader2,
            options?.headers
          ]),
          stream: params.stream ?? false
        });
      }
      /**
       * Send a structured list of input messages with text and/or image content, along with an expected `output_config.format` and
       * the response will be automatically parsed and available in the `parsed_output` property of the message.
       *
       * @example
       * ```ts
       * const message = await client.messages.parse({
       *   model: 'claude-sonnet-4-5-20250929',
       *   max_tokens: 1024,
       *   messages: [{ role: 'user', content: 'What is 2+2?' }],
       *   output_config: {
       *     format: zodOutputFormat(z.object({ answer: z.number() })),
       *   },
       * });
       *
       * console.log(message.parsed_output?.answer); // 4
       * ```
       */
      parse(params, options) {
        return this.create(params, options).then((message) => parseMessage(message, params, { logger: this._client.logger ?? console }));
      }
      /**
       * Create a Message stream.
       *
       * If `output_config.format` is provided with a parseable format (like `zodOutputFormat()`),
       * the final message will include a `parsed_output` property with the parsed content.
       *
       * @example
       * ```ts
       * const stream = client.messages.stream({
       *   model: 'claude-sonnet-4-5-20250929',
       *   max_tokens: 1024,
       *   messages: [{ role: 'user', content: 'What is 2+2?' }],
       *   output_config: {
       *     format: zodOutputFormat(z.object({ answer: z.number() })),
       *   },
       * });
       *
       * const message = await stream.finalMessage();
       * console.log(message.parsed_output?.answer); // 4
       * ```
       */
      stream(body, options) {
        return MessageStream.createMessage(this, body, options, { logger: this._client.logger ?? console });
      }
      /**
       * Count the number of tokens in a Message.
       *
       * The Token Count API can be used to count the number of tokens in a Message,
       * including tools, images, and documents, without creating it.
       *
       * Learn more about token counting in our
       * [user guide](https://platform.claude.com/docs/en/build-with-claude/token-counting)
       *
       * @example
       * ```ts
       * const messageTokensCount =
       *   await client.messages.countTokens({
       *     messages: [{ content: 'Hello, world', role: 'user' }],
       *     model: 'claude-opus-4-6',
       *   });
       * ```
       */
      countTokens(params, options) {
        const { user_profile_id, ...body } = params;
        return this._client.post("/v1/messages/count_tokens", {
          body,
          ...options,
          headers: buildHeaders([
            { ...user_profile_id != null ? { "anthropic-user-profile-id": user_profile_id } : void 0 },
            options?.headers
          ])
        });
      }
    };
    DEPRECATED_MODELS2 = {};
    MODELS_TO_WARN_WITH_THINKING_ENABLED2 = ["claude-mythos-preview", "claude-opus-4-6"];
    Messages2.Batches = Batches2;
  }
});

// node_modules/@anthropic-ai/sdk/resources/models.mjs
var Models2;
var init_models2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/models.mjs"() {
    init_resource();
    init_pagination();
    init_headers();
    init_path();
    Models2 = class extends APIResource {
      /**
       * Get a specific model.
       *
       * The Models API response can be used to determine information about a specific
       * model or resolve a model alias to a model ID.
       */
      retrieve(modelID, params = {}, options) {
        const { betas } = params ?? {};
        return this._client.get(path`/v1/models/${modelID}`, {
          ...options,
          headers: buildHeaders([
            { ...betas?.toString() != null ? { "anthropic-beta": betas?.toString() } : void 0 },
            options?.headers
          ])
        });
      }
      /**
       * List available models.
       *
       * The Models API response can be used to determine which models are available for
       * use in the API. More recently released models are listed first.
       */
      list(params = {}, options) {
        const { betas, ...query } = params ?? {};
        return this._client.getAPIList("/v1/models", Page, {
          query,
          ...options,
          headers: buildHeaders([
            { ...betas?.toString() != null ? { "anthropic-beta": betas?.toString() } : void 0 },
            options?.headers
          ])
        });
      }
    };
  }
});

// node_modules/@anthropic-ai/sdk/resources/index.mjs
var init_resources2 = __esm({
  "node_modules/@anthropic-ai/sdk/resources/index.mjs"() {
    init_shared();
    init_beta();
    init_completions();
    init_messages2();
    init_models2();
  }
});

// node_modules/@anthropic-ai/sdk/client.mjs
var _BaseAnthropic_instances, _a, _BaseAnthropic_encoder, _BaseAnthropic_baseURLOverridden, HUMAN_PROMPT, AI_PROMPT, BaseAnthropic, Anthropic;
var init_client = __esm({
  "node_modules/@anthropic-ai/sdk/client.mjs"() {
    init_tslib();
    init_uuid();
    init_values();
    init_sleep();
    init_errors();
    init_detect_platform();
    init_request_signal();
    init_shims();
    init_request_options();
    init_query();
    init_version();
    init_error();
    init_types();
    init_token_cache();
    init_credential_chain();
    init_middleware();
    init_pagination();
    init_uploads2();
    init_resources2();
    init_api_promise();
    init_completions();
    init_models2();
    init_beta();
    init_messages2();
    init_detect_platform();
    init_headers();
    init_env();
    init_log();
    init_values();
    HUMAN_PROMPT = "\\n\\nHuman:";
    AI_PROMPT = "\\n\\nAssistant:";
    BaseAnthropic = class {
      /**
       * The active credential provider. Default credential resolution runs once
       * at construction time. If it fails, the error is surfaced on every
       * request and the client must be reconstructed — there is no retry path.
       *
       * Clones returned by {@link withOptions} share the parent's auth state
       * (provider, token cache, pending resolution, and any resolution error)
       * unless the caller passes an explicit `apiKey`, `authToken`,
       * `credentials`, `config`, or `profile` override.
       */
      get credentials() {
        return this._authState.provider;
      }
      /**
       * API Client for interfacing with the Anthropic API.
       *
       * @param {string | null | undefined} [opts.apiKey=process.env['ANTHROPIC_API_KEY'] ?? null]
       * @param {string | null | undefined} [opts.authToken=process.env['ANTHROPIC_AUTH_TOKEN'] ?? null]
       * @param {string | null | undefined} [opts.webhookKey=process.env['ANTHROPIC_WEBHOOK_SIGNING_KEY'] ?? null]
       * @param {string} [opts.baseURL=process.env['ANTHROPIC_BASE_URL'] ?? https://api.anthropic.com] - Override the default base URL for the API.
       * @param {number} [opts.timeout=10 minutes] - The maximum amount of time (in milliseconds) the client will wait for a response before timing out.
       * @param {MergedRequestInit} [opts.fetchOptions] - Additional `RequestInit` options to be passed to `fetch` calls.
       * @param {Fetch} [opts.fetch] - Specify a custom `fetch` function implementation.
       * @param {number} [opts.maxRetries=2] - The maximum number of times the client will retry a request.
       * @param {HeadersLike} opts.defaultHeaders - Default headers to include with every request to the API.
       * @param {Record<string, string | undefined>} opts.defaultQuery - Default query parameters to include with every request to the API.
       * @param {boolean} [opts.dangerouslyAllowBrowser=false] - By default, client-side use of this library is not allowed, as it risks exposing your secret API credentials to attackers.
       */
      constructor({ baseURL = readEnv("ANTHROPIC_BASE_URL"), apiKey, authToken, webhookKey = readEnv("ANTHROPIC_WEBHOOK_SIGNING_KEY") ?? null, ...opts } = {}) {
        _BaseAnthropic_instances.add(this);
        this._requestAuthFlags = /* @__PURE__ */ new WeakMap();
        _BaseAnthropic_encoder.set(this, void 0);
        if (apiKey === void 0) {
          apiKey = opts.profile != null ? null : readEnv("ANTHROPIC_API_KEY") ?? null;
        }
        if (authToken === void 0) {
          authToken = opts.profile != null ? null : readEnv("ANTHROPIC_AUTH_TOKEN") ?? null;
        }
        if (opts.profile != null && (opts.credentials != null || opts.config != null)) {
          throw new TypeError("Pass at most one of `profile`, `credentials`, or `config`.");
        }
        const options = {
          apiKey,
          authToken,
          webhookKey,
          ...opts,
          baseURL: baseURL || `https://api.anthropic.com`
        };
        if (!options.dangerouslyAllowBrowser && isRunningInBrowser()) {
          throw new AnthropicError("It looks like you're running in a browser-like environment.\n\nThis is disabled by default, as it risks exposing your secret API credentials to attackers.\nIf you understand the risks and have appropriate mitigations in place,\nyou can set the `dangerouslyAllowBrowser` option to `true`, e.g.,\n\nnew Anthropic({ apiKey, dangerouslyAllowBrowser: true });\n");
        }
        this.baseURL = options.baseURL;
        this._baseURLIsExplicit = opts.__baseURLIsExplicit ?? !!baseURL;
        this.timeout = options.timeout ?? _a.DEFAULT_TIMEOUT;
        this.logger = options.logger ?? console;
        this.logLevel = defaultLogLevel;
        this.logLevel = parseLogLevel(options.logLevel, "ClientOptions.logLevel", loggerFor(this)) ?? parseLogLevel(readEnv("ANTHROPIC_LOG"), "process.env['ANTHROPIC_LOG']", loggerFor(this)) ?? defaultLogLevel;
        this.fetchOptions = options.fetchOptions;
        this.maxRetries = options.maxRetries ?? 2;
        this.fetch = options.fetch ?? getDefaultFetch();
        __classPrivateFieldSet(this, _BaseAnthropic_encoder, FallbackEncoder, "f");
        this.middleware = [...options.middleware ?? []];
        const customHeadersEnv = readEnv("ANTHROPIC_CUSTOM_HEADERS");
        if (customHeadersEnv) {
          const parsed = {};
          for (const line of customHeadersEnv.split("\n")) {
            const colon = line.indexOf(":");
            if (colon >= 0) {
              parsed[line.substring(0, colon).trim()] = line.substring(colon + 1).trim();
            }
          }
          options.defaultHeaders = { ...parsed, ...options.defaultHeaders };
        }
        const inherited = opts.__auth;
        delete options.__auth;
        delete options.__baseURLIsExplicit;
        this._options = options;
        this.apiKey = typeof apiKey === "string" ? apiKey : null;
        this.authToken = authToken;
        this.webhookKey = webhookKey;
        if (inherited) {
          this._authState = inherited;
          if (!this._baseURLIsExplicit && inherited.baseURL) {
            this.baseURL = inherited.baseURL;
          }
        } else {
          this._authState = { provider: null, tokenCache: null, resolution: null, error: null, extraHeaders: {} };
          if (this.apiKey == null && this.authToken == null) {
            const credentials = options.credentials ?? null;
            if (credentials) {
              this._authState.provider = credentials;
              this._authState.tokenCache = this._makeTokenCache(credentials);
            } else if (options.config != null) {
              const result = resolveCredentialsFromConfig(options.config, this._credentialResolverOptions());
              this._authState.provider = result.provider;
              this._authState.tokenCache = this._makeTokenCache(result.provider);
              this._authState.extraHeaders = result.extraHeaders;
              this._applyCredentialBaseURL(result.baseURL);
            } else if (options.profile != null) {
              this._authState.resolution = this._resolveDefaultCredentials(options.profile);
            } else if (this._shouldResolveDefaultCredentials()) {
              this._authState.resolution = this._resolveDefaultCredentials();
            }
          }
        }
      }
      /**
       * Whether to lazily resolve auth from the default credential chain when no
       * explicit auth is configured. Called once from the constructor, so
       * overrides must not depend on subclass instance state. Subclasses that
       * bring their own auth scheme return false so unrelated local credentials
       * are never resolved or allowed to supply a base URL.
       */
      _shouldResolveDefaultCredentials() {
        return true;
      }
      /**
       * Stores a profile/config-supplied base URL on the shared auth state and, if
       * the caller did not pin `baseURL` via constructor option or env, adopts it
       * as this client's outbound API host. Precedence: ctor opt > env > profile >
       * hardcoded default.
       */
      _applyCredentialBaseURL(baseURL) {
        if (!baseURL)
          return;
        const normalized = baseURL.replace(/\/+$/, "");
        this._authState.baseURL = normalized;
        if (!this._baseURLIsExplicit) {
          this.baseURL = normalized;
        }
      }
      /**
       * Options bag passed into the credential chain. `baseURL` here is only the
       * fallback host for the token-exchange POST when the config itself omits
       * `base_url`; the chain returns the config's own `base_url` (if any) on
       * {@link CredentialResult.baseURL}, which {@link _applyCredentialBaseURL}
       * then adopts for outbound API requests. The two are deliberately decoupled
       * so this fallback never round-trips into precedence.
       */
      _credentialResolverOptions() {
        return {
          baseURL: this.baseURL,
          fetch: this._credentialsFetch(),
          userAgent: this.getUserAgent(),
          onCacheWriteError: (err) => {
            loggerFor(this).debug("credential cache write failed (best-effort)", err);
          },
          onSafetyWarning: (msg) => {
            loggerFor(this).warn(msg);
          }
        };
      }
      /**
       * A `Fetch` for first-party credential token-exchange requests (OIDC
       * federation jwt-bearer grants, user-OAuth refresh grants) that routes
       * through this client's middleware chain, so middleware observes token
       * traffic like any other request. Only client-level middleware applies:
       * a minted token is shared across requests, so attributing the exchange
       * to any one request's per-request middleware would be arbitrary. For the
       * same reason, `ctx.options` is undefined for these requests.
       */
      _credentialsFetch() {
        return wrapFetchWithMiddleware(this.fetch, this.middleware, void 0, this);
      }
      _makeTokenCache(provider2) {
        return new TokenCache(provider2, (err) => {
          loggerFor(this).debug("advisory token refresh failed; serving cached token", err);
        });
      }
      /**
       * Create a new client instance re-using the same options given to the current client with optional overriding.
       */
      withOptions(options) {
        const overridesStructuredAuth = "credentials" in options || "config" in options || "profile" in options;
        const overridesAuth = "apiKey" in options || "authToken" in options || overridesStructuredAuth;
        const internal = {
          ...this._options,
          // Only forward baseURL when the caller (or env) explicitly chose it.
          // For a non-explicit parent, this.baseURL may have been mutated to the
          // profile-resolved host; pinning that as the clone's options.baseURL
          // would make _options on the clone misreport caller intent and would
          // leave the clone stuck on the parent's host across an auth override.
          // The clone instead receives the construction-time value via
          // ...this._options above and re-adopts the profile host through the
          // shared _authState.baseURL + __baseURLIsExplicit=false path.
          ...this._baseURLIsExplicit ? { baseURL: this.baseURL } : {},
          maxRetries: this.maxRetries,
          timeout: this.timeout,
          logger: this.logger,
          logLevel: this.logLevel,
          fetch: this.fetch,
          fetchOptions: this.fetchOptions,
          middleware: this.middleware,
          apiKey: this.apiKey,
          authToken: this.authToken,
          webhookKey: this.webhookKey,
          // credentials: this.credentials is a no-op when __auth is shared (the
          // ctor takes the inherited path and ignores options.credentials); when
          // overridesAuth is true via apiKey/authToken only, it lets the clone
          // build a fresh TokenCache around the parent's provider.
          credentials: this.credentials,
          // When the caller passes a structured-credential override, drop inherited
          // structured-credential options so only `...options` supplies them —
          // otherwise an inherited `credentials`/`config`/`profile` would trip the
          // mutual-exclusion check or precedence over the override.
          ...overridesStructuredAuth ? { credentials: void 0, config: void 0, profile: void 0 } : {},
          ...options,
          // Always set __auth so any stale value from ...this._options is
          // overwritten. undefined means "build fresh auth from these options".
          __auth: overridesAuth ? void 0 : this._authState,
          __baseURLIsExplicit: "baseURL" in options ? true : this._baseURLIsExplicit
        };
        return new this.constructor(internal);
      }
      /**
       * Lazily resolves credentials from config files or environment variables.
       * Called once from the constructor when no explicit auth is provided, or
       * when an explicit `profile` was passed (in which case a missing/unresolved
       * profile is surfaced as an error instead of falling through to "no auth").
       * The returned promise is stored and awaited on the first request.
       */
      async _resolveDefaultCredentials(profile) {
        try {
          const result = await defaultCredentials(this._credentialResolverOptions(), profile);
          if (result) {
            this._authState.provider = result.provider;
            this._authState.tokenCache = this._makeTokenCache(result.provider);
            this._authState.extraHeaders = result.extraHeaders;
            this._applyCredentialBaseURL(result.baseURL);
          } else if (profile != null) {
            throw new AnthropicError(`Profile "${profile}" could not be resolved (no <config_dir>/configs/${profile}.json found).`);
          }
        } catch (err) {
          this._authState.error = err;
        } finally {
          this._authState.resolution = null;
        }
      }
      defaultQuery() {
        return this._options.defaultQuery;
      }
      validateHeaders({ values, nulls }) {
        if (values.get("x-api-key") || values.get("authorization")) {
          return;
        }
        if (this._authState.error) {
          throw this._authState.error;
        }
        if (this._authState.tokenCache || this._authState.resolution) {
          return;
        }
        if (this.apiKey && values.get("x-api-key")) {
          return;
        }
        if (nulls.has("x-api-key")) {
          return;
        }
        if (this.authToken && values.get("authorization")) {
          return;
        }
        if (nulls.has("authorization")) {
          return;
        }
        throw new Error('Could not resolve authentication method. Expected one of apiKey, authToken, credentials, config, or profile to be set. Or for one of the "X-Api-Key" or "Authorization" headers to be explicitly omitted');
      }
      _authFlags(opts) {
        let flags = this._requestAuthFlags.get(opts);
        if (!flags) {
          flags = { usedTokenCache: false, didRefreshFor401: false };
          this._requestAuthFlags.set(opts, flags);
        }
        return flags;
      }
      async authHeaders(opts) {
        if (this._authState.resolution) {
          await this._authState.resolution;
        }
        if (this._authState.error) {
          return void 0;
        }
        if (this._authState.tokenCache && this.apiKey == null) {
          const token2 = await this._authState.tokenCache.getToken();
          this._authFlags(opts).usedTokenCache = true;
          return buildHeaders([{ Authorization: `Bearer ${token2}` }]);
        }
        return buildHeaders([await this.apiKeyAuth(opts), await this.bearerAuth(opts)]);
      }
      async apiKeyAuth(opts) {
        if (this.apiKey == null) {
          return void 0;
        }
        return buildHeaders([{ "X-Api-Key": this.apiKey }]);
      }
      async bearerAuth(opts) {
        if (this.authToken == null) {
          return void 0;
        }
        return buildHeaders([{ Authorization: `Bearer ${this.authToken}` }]);
      }
      stringifyQuery(query) {
        return stringifyQuery(query);
      }
      getUserAgent() {
        return `Anthropic/JS ${VERSION}`;
      }
      defaultIdempotencyKey() {
        return `stainless-node-retry-${uuid4()}`;
      }
      makeStatusError(status, error, message, headers) {
        return APIError.generate(status, error, message, headers);
      }
      buildURL(path6, query, defaultBaseURL) {
        const baseURL = !__classPrivateFieldGet(this, _BaseAnthropic_instances, "m", _BaseAnthropic_baseURLOverridden).call(this) && defaultBaseURL || this.baseURL;
        const url = isAbsoluteURL(path6) ? new URL(path6) : new URL(baseURL + (baseURL.endsWith("/") && path6.startsWith("/") ? path6.slice(1) : path6));
        const defaultQuery = this.defaultQuery();
        const pathQuery = Object.fromEntries(url.searchParams);
        if (!isEmptyObj(defaultQuery) || !isEmptyObj(pathQuery)) {
          query = { ...pathQuery, ...defaultQuery, ...query };
        }
        if (typeof query === "object" && query && !Array.isArray(query)) {
          url.search = this.stringifyQuery(query);
        }
        return url.toString();
      }
      _calculateNonstreamingTimeout(maxTokens) {
        const defaultTimeout = 10 * 60;
        const expectedTimeout = 60 * 60 * maxTokens / 128e3;
        if (expectedTimeout > defaultTimeout) {
          throw new AnthropicError("Streaming is required for operations that may take longer than 10 minutes. See https://github.com/anthropics/anthropic-sdk-typescript#streaming-responses for more details");
        }
        return defaultTimeout * 1e3;
      }
      /**
       * Used as a callback for mutating the given `FinalRequestOptions` object.
       */
      async prepareOptions(options) {
      }
      /**
       * Used as a callback for mutating the given `RequestInit` object.
       *
       * This is useful for cases where you want to add certain headers based off of
       * the request properties, e.g. `method` or `url`.
       *
       * Runs after all middleware (including {@link backendMiddleware}),
       * immediately before each underlying fetch call, so it sees exactly what
       * goes over the wire. Middleware may replay a request by calling `next()`
       * more than once, so this hook can run multiple times per attempt:
       * overrides must be idempotent and overwrite headers from a previous
       * invocation rather than append to them.
       */
      async prepareRequest(request, { url, options }) {
        if (this._authState.tokenCache && this.apiKey == null) {
          const headers = request.headers instanceof Headers ? request.headers : new Headers(request.headers);
          for (const [k, v] of Object.entries(this._authState.extraHeaders)) {
            if (!headers.has(k))
              headers.set(k, v);
          }
          const existing = headers.get("anthropic-beta")?.split(",").map((s) => s.trim());
          if (!existing?.includes(OAUTH_API_BETA_HEADER)) {
            headers.append("anthropic-beta", OAUTH_API_BETA_HEADER);
          }
          request.headers = headers;
        }
      }
      /**
       * Internal {@link Middleware} composed innermost in the chain — inside both
       * client-level and per-request middleware, immediately around the underlying
       * `fetch`. Subclasses for third-party backends override this to adapt the
       * canonical Anthropic-shaped request to the backend's wire shape (URL/body
       * rewriting, request signing) and to normalize the wire response back to the
       * canonical shape (e.g. AWS EventStream to SSE).
       *
       * Running inside the user's middleware means user middleware always observes
       * canonical Anthropic-shaped traffic, and the adaptation re-runs (e.g.
       * re-signs) on every `next()` invocation, covering whatever the middleware
       * mutated.
       *
       * Errors thrown here follow the middleware error policy: they propagate to
       * the caller as-is — no retries, no `APIConnectionError` wrapping — unless
       * retryable (see {@link Middleware}); throw a `RetryableError` to opt into
       * the retry path.
       */
      backendMiddleware() {
        return [];
      }
      get(path6, opts) {
        return this.methodRequest("get", path6, opts);
      }
      post(path6, opts) {
        return this.methodRequest("post", path6, opts);
      }
      patch(path6, opts) {
        return this.methodRequest("patch", path6, opts);
      }
      put(path6, opts) {
        return this.methodRequest("put", path6, opts);
      }
      delete(path6, opts) {
        return this.methodRequest("delete", path6, opts);
      }
      methodRequest(method, path6, opts) {
        return this.request(Promise.resolve(opts).then((opts2) => {
          return { method, path: path6, ...opts2 };
        }));
      }
      request(options, remainingRetries = null) {
        return new APIPromise(this, this.makeRequest(options, remainingRetries, void 0));
      }
      async makeRequest(optionsInput, retriesRemaining, retryOfRequestLogID) {
        const options = await optionsInput;
        const maxRetries = options.maxRetries ?? this.maxRetries;
        if (retriesRemaining == null) {
          retriesRemaining = maxRetries;
          this._requestAuthFlags.delete(options);
        }
        await this.prepareOptions(options);
        const { req, url, timeout } = await this.buildRequest(options, {
          retryCount: maxRetries - retriesRemaining
        });
        const requestLogID = "log_" + (Math.random() * (1 << 24) | 0).toString(16).padStart(6, "0");
        const retryLogStr = retryOfRequestLogID === void 0 ? "" : `, retryOf: ${retryOfRequestLogID}`;
        const startTime = Date.now();
        if (options.signal?.aborted) {
          throw new APIUserAbortError();
        }
        const controller = new AbortController();
        const response = await this.fetchWithTimeout(url, req, timeout, controller, options, {
          requestLogID,
          retryOfRequestLogID
        }).catch(castToError);
        const headersTime = Date.now();
        if (response instanceof globalThis.Error) {
          releaseRequestSignal(controller);
          const retryMessage = `retrying, ${retriesRemaining} attempts remaining`;
          if (options.signal?.aborted) {
            throw new APIUserAbortError();
          }
          const isTimeout = isAbortError(response) || /timed? ?out/i.test(String(response) + ("cause" in response ? String(response.cause) : ""));
          const hasMiddleware = this.middleware.length > 0 || !!options.middleware?.length || this.backendMiddleware().length > 0;
          if (hasMiddleware && !isTimeout && !isRetryableError(response)) {
            loggerFor(this).info(`[${requestLogID}] middleware error (not retryable)`);
            loggerFor(this).debug(`[${requestLogID}] middleware error (not retryable)`, formatRequestDetails({
              retryOfRequestLogID,
              url,
              durationMs: headersTime - startTime,
              message: response.message
            }));
            throw response;
          }
          if (retriesRemaining) {
            loggerFor(this).info(`[${requestLogID}] connection ${isTimeout ? "timed out" : "failed"} - ${retryMessage}`);
            loggerFor(this).debug(`[${requestLogID}] connection ${isTimeout ? "timed out" : "failed"} (${retryMessage})`, formatRequestDetails({
              retryOfRequestLogID,
              url,
              durationMs: headersTime - startTime,
              message: response.message
            }));
            return this.retryRequest(options, retriesRemaining, retryOfRequestLogID ?? requestLogID);
          }
          loggerFor(this).info(`[${requestLogID}] connection ${isTimeout ? "timed out" : "failed"} - error; no more retries left`);
          loggerFor(this).debug(`[${requestLogID}] connection ${isTimeout ? "timed out" : "failed"} (error; no more retries left)`, formatRequestDetails({
            retryOfRequestLogID,
            url,
            durationMs: headersTime - startTime,
            message: response.message
          }));
          if (isTimeout) {
            throw new APIConnectionTimeoutError();
          }
          if (hasMiddleware && !isFetchOriginError(response)) {
            throw response;
          }
          throw new APIConnectionError({ cause: response });
        }
        const specialHeaders = [...response.headers.entries()].filter(([name]) => name === "request-id").map(([name, value]) => ", " + name + ": " + JSON.stringify(value)).join("");
        const responseInfo = `[${requestLogID}${retryLogStr}${specialHeaders}] ${req.method} ${url} ${response.ok ? "succeeded" : "failed"} with status ${response.status} in ${headersTime - startTime}ms`;
        if (!response.ok) {
          const shouldRetry = await this.shouldRetry(response, options);
          if (retriesRemaining && shouldRetry) {
            const retryMessage2 = `retrying, ${retriesRemaining} attempts remaining`;
            await CancelReadableStream(response.body);
            releaseRequestSignal(controller);
            loggerFor(this).info(`${responseInfo} - ${retryMessage2}`);
            loggerFor(this).debug(`[${requestLogID}] response error (${retryMessage2})`, formatRequestDetails({
              retryOfRequestLogID,
              url: response.url,
              status: response.status,
              headers: response.headers,
              durationMs: headersTime - startTime
            }));
            return this.retryRequest(options, retriesRemaining, retryOfRequestLogID ?? requestLogID, response.headers);
          }
          const retryMessage = shouldRetry ? `error; no more retries left` : `error; not retryable`;
          loggerFor(this).info(`${responseInfo} - ${retryMessage}`);
          const errText = await response.text().catch((err2) => castToError(err2).message);
          const errJSON = safeJSON(errText);
          const errMessage = errJSON ? void 0 : errText;
          loggerFor(this).debug(`[${requestLogID}] response error (${retryMessage})`, formatRequestDetails({
            retryOfRequestLogID,
            url: response.url,
            status: response.status,
            headers: response.headers,
            message: errMessage,
            durationMs: Date.now() - startTime
          }));
          releaseRequestSignal(controller);
          const err = this.makeStatusError(response.status, errJSON, errMessage, response.headers);
          throw err;
        }
        loggerFor(this).info(responseInfo);
        loggerFor(this).debug(`[${requestLogID}] response start`, formatRequestDetails({
          retryOfRequestLogID,
          url: response.url,
          status: response.status,
          headers: response.headers,
          durationMs: headersTime - startTime
        }));
        armAbandonmentBackstop(response.body ?? response, controller);
        return { response, options, controller, requestLogID, retryOfRequestLogID, startTime };
      }
      getAPIList(path6, Page2, opts) {
        return this.requestAPIList(Page2, opts && "then" in opts ? opts.then((opts2) => ({ method: "get", path: path6, ...opts2 })) : { method: "get", path: path6, ...opts });
      }
      requestAPIList(Page2, options) {
        const request = this.makeRequest(options, null, void 0);
        return new PagePromise(this, request, Page2);
      }
      async fetchWithTimeout(url, init, ms, controller, requestOptions, logCtx) {
        const { signal, method, ...options } = init || {};
        const abort = this._makeAbort(controller);
        if (signal) {
          signal.addEventListener("abort", abort, { once: true });
          registerRequestSignalCleanup(controller, signal, abort);
        }
        const isReadableBody = globalThis.ReadableStream && options.body instanceof globalThis.ReadableStream || typeof options.body === "object" && options.body !== null && Symbol.asyncIterator in options.body;
        const fetchOptions = {
          signal: controller.signal,
          ...isReadableBody ? { duplex: "half" } : {},
          method: "GET",
          ...options
        };
        if (method) {
          fetchOptions.method = method.toUpperCase();
        }
        const baseFetch = this.fetch;
        const timedFetch = async (innerUrl, innerInit) => {
          const timeout = setTimeout(abort, ms);
          try {
            return await baseFetch.call(void 0, innerUrl, innerInit);
          } finally {
            clearTimeout(timeout);
          }
        };
        const innerFetch = requestOptions === void 0 ? timedFetch : (async (innerUrl, innerInit = {}) => {
          const innerUrlStr = typeof innerUrl === "string" ? innerUrl : innerUrl instanceof URL ? innerUrl.href : innerUrl.url;
          innerInit.headers = innerInit.headers instanceof Headers ? innerInit.headers : new Headers(innerInit.headers);
          await this.prepareRequest(innerInit, { url: innerUrlStr, options: requestOptions });
          if (logCtx) {
            loggerFor(this).debug(`[${logCtx.requestLogID}] sending request`, formatRequestDetails({
              retryOfRequestLogID: logCtx.retryOfRequestLogID,
              method: innerInit.method,
              url: innerUrlStr,
              options: requestOptions,
              headers: innerInit.headers
            }));
          }
          return timedFetch(innerUrl, innerInit);
        });
        const requestMiddleware = requestOptions?.middleware;
        const backendMiddleware = this.backendMiddleware();
        const allMiddleware = requestMiddleware?.length || backendMiddleware.length ? [...this.middleware, ...requestMiddleware ?? [], ...backendMiddleware] : this.middleware;
        return await wrapFetchWithMiddleware(innerFetch, allMiddleware, requestOptions, this)(url, fetchOptions);
      }
      async shouldRetry(response, options) {
        const flags = this._authFlags(options);
        if (response.status === 401 && this._authState.tokenCache && flags.usedTokenCache && !flags.didRefreshFor401) {
          flags.didRefreshFor401 = true;
          this._authState.tokenCache.invalidate();
          return true;
        }
        const shouldRetryHeader = response.headers.get("x-should-retry");
        if (shouldRetryHeader === "true")
          return true;
        if (shouldRetryHeader === "false")
          return false;
        if (response.status === 408)
          return true;
        if (response.status === 409)
          return true;
        if (response.status === 429)
          return true;
        if (response.status >= 500)
          return true;
        return false;
      }
      async retryRequest(options, retriesRemaining, requestLogID, responseHeaders) {
        let timeoutMillis;
        const retryAfterMillisHeader = responseHeaders?.get("retry-after-ms");
        if (retryAfterMillisHeader) {
          const timeoutMs = parseFloat(retryAfterMillisHeader);
          if (!Number.isNaN(timeoutMs)) {
            timeoutMillis = timeoutMs;
          }
        }
        const retryAfterHeader = responseHeaders?.get("retry-after");
        if (retryAfterHeader && !timeoutMillis) {
          const timeoutSeconds = parseFloat(retryAfterHeader);
          if (!Number.isNaN(timeoutSeconds)) {
            timeoutMillis = timeoutSeconds * 1e3;
          } else {
            timeoutMillis = Date.parse(retryAfterHeader) - Date.now();
          }
        }
        if (timeoutMillis === void 0) {
          const maxRetries = options.maxRetries ?? this.maxRetries;
          timeoutMillis = this.calculateDefaultRetryTimeoutMillis(retriesRemaining, maxRetries);
        }
        await sleep(timeoutMillis);
        return this.makeRequest(options, retriesRemaining - 1, requestLogID);
      }
      calculateDefaultRetryTimeoutMillis(retriesRemaining, maxRetries) {
        const initialRetryDelay = 0.5;
        const maxRetryDelay = 8;
        const numRetries = maxRetries - retriesRemaining;
        const sleepSeconds = Math.min(initialRetryDelay * Math.pow(2, numRetries), maxRetryDelay);
        const jitter2 = 1 - Math.random() * 0.25;
        return sleepSeconds * jitter2 * 1e3;
      }
      calculateNonstreamingTimeout(maxTokens, maxNonstreamingTokens) {
        const maxTime = 60 * 60 * 1e3;
        const defaultTime = 60 * 10 * 1e3;
        const expectedTime = maxTime * maxTokens / 128e3;
        if (expectedTime > defaultTime || maxNonstreamingTokens != null && maxTokens > maxNonstreamingTokens) {
          throw new AnthropicError("Streaming is required for operations that may take longer than 10 minutes. See https://github.com/anthropics/anthropic-sdk-typescript#long-requests for more details");
        }
        return defaultTime;
      }
      async buildRequest(inputOptions, { retryCount = 0 } = {}) {
        const options = { ...inputOptions };
        const { method, path: path6, query, defaultBaseURL } = options;
        if (this._authState.resolution) {
          await this._authState.resolution;
        }
        if (!this._baseURLIsExplicit && this._authState.baseURL && this.baseURL !== this._authState.baseURL) {
          this.baseURL = this._authState.baseURL;
        }
        const url = this.buildURL(path6, query, defaultBaseURL);
        if ("timeout" in options)
          validatePositiveInteger("timeout", options.timeout);
        options.timeout = options.timeout ?? this.timeout;
        const { bodyHeaders, body } = this.buildBody({ options });
        const reqHeaders = await this.buildHeaders({ options: inputOptions, method, bodyHeaders, retryCount });
        const req = {
          method,
          headers: reqHeaders,
          ...options.signal && { signal: options.signal },
          ...globalThis.ReadableStream && body instanceof globalThis.ReadableStream && { duplex: "half" },
          ...body && { body },
          ...this.fetchOptions ?? {},
          ...options.fetchOptions ?? {}
        };
        return { req, url, timeout: options.timeout };
      }
      async buildHeaders({ options, method, bodyHeaders, retryCount }) {
        let idempotencyHeaders = {};
        if (this.idempotencyHeader && method !== "get") {
          if (!options.idempotencyKey)
            options.idempotencyKey = this.defaultIdempotencyKey();
          idempotencyHeaders[this.idempotencyHeader] = options.idempotencyKey;
        }
        const headers = buildHeaders([
          idempotencyHeaders,
          {
            Accept: "application/json",
            "User-Agent": this.getUserAgent(),
            "X-Stainless-Retry-Count": String(retryCount),
            ...options.timeout ? { "X-Stainless-Timeout": String(Math.trunc(options.timeout / 1e3)) } : {},
            ...getPlatformHeaders(),
            ...this._options.dangerouslyAllowBrowser ? { "anthropic-dangerous-direct-browser-access": "true" } : void 0,
            "anthropic-version": "2023-06-01"
          },
          await this.authHeaders(options),
          this._options.defaultHeaders,
          bodyHeaders,
          options.headers
        ]);
        this.validateHeaders(headers);
        return headers.values;
      }
      _makeAbort(controller) {
        return () => controller.abort();
      }
      buildBody({ options: { body, headers: rawHeaders } }) {
        if (!body) {
          return { bodyHeaders: void 0, body: void 0 };
        }
        const headers = buildHeaders([rawHeaders]);
        if (
          // Pass raw type verbatim
          ArrayBuffer.isView(body) || body instanceof ArrayBuffer || body instanceof DataView || typeof body === "string" && // Preserve legacy string encoding behavior for now
          headers.values.has("content-type") || // `Blob` is superset of `File`
          globalThis.Blob && body instanceof globalThis.Blob || // `FormData` -> `multipart/form-data`
          body instanceof FormData || // `URLSearchParams` -> `application/x-www-form-urlencoded`
          body instanceof URLSearchParams || // Send chunked stream (each chunk has own `length`)
          globalThis.ReadableStream && body instanceof globalThis.ReadableStream
        ) {
          return { bodyHeaders: void 0, body };
        } else if (typeof body === "object" && (Symbol.asyncIterator in body || Symbol.iterator in body && "next" in body && typeof body.next === "function")) {
          return { bodyHeaders: void 0, body: ReadableStreamFrom(body) };
        } else if (typeof body === "object" && headers.values.get("content-type") === "application/x-www-form-urlencoded") {
          return {
            bodyHeaders: { "content-type": "application/x-www-form-urlencoded" },
            body: this.stringifyQuery(body)
          };
        } else {
          return __classPrivateFieldGet(this, _BaseAnthropic_encoder, "f").call(this, { body, headers });
        }
      }
    };
    _a = BaseAnthropic, _BaseAnthropic_encoder = /* @__PURE__ */ new WeakMap(), _BaseAnthropic_instances = /* @__PURE__ */ new WeakSet(), _BaseAnthropic_baseURLOverridden = function _BaseAnthropic_baseURLOverridden2() {
      return this.baseURL !== "https://api.anthropic.com";
    };
    BaseAnthropic.Anthropic = _a;
    BaseAnthropic.HUMAN_PROMPT = HUMAN_PROMPT;
    BaseAnthropic.AI_PROMPT = AI_PROMPT;
    BaseAnthropic.DEFAULT_TIMEOUT = 6e5;
    BaseAnthropic.AnthropicError = AnthropicError;
    BaseAnthropic.APIError = APIError;
    BaseAnthropic.APIConnectionError = APIConnectionError;
    BaseAnthropic.APIConnectionTimeoutError = APIConnectionTimeoutError;
    BaseAnthropic.APIUserAbortError = APIUserAbortError;
    BaseAnthropic.NotFoundError = NotFoundError;
    BaseAnthropic.ConflictError = ConflictError;
    BaseAnthropic.RateLimitError = RateLimitError;
    BaseAnthropic.BadRequestError = BadRequestError;
    BaseAnthropic.AuthenticationError = AuthenticationError;
    BaseAnthropic.InternalServerError = InternalServerError;
    BaseAnthropic.PermissionDeniedError = PermissionDeniedError;
    BaseAnthropic.UnprocessableEntityError = UnprocessableEntityError;
    BaseAnthropic.toFile = toFile;
    Anthropic = class extends BaseAnthropic {
      constructor() {
        super(...arguments);
        this.completions = new Completions(this);
        this.messages = new Messages2(this);
        this.models = new Models2(this);
        this.beta = new Beta(this);
      }
    };
    Anthropic.Completions = Completions;
    Anthropic.Messages = Messages2;
    Anthropic.Models = Models2;
    Anthropic.Beta = Beta;
  }
});

// node_modules/@anthropic-ai/sdk/lib/middleware.mjs
var encoder;
var init_middleware2 = __esm({
  "node_modules/@anthropic-ai/sdk/lib/middleware.mjs"() {
    init_error();
    init_streaming();
    init_errors();
    init_headers();
    init_stainless_helper_header();
    init_values();
    init_request_options();
    encoder = new TextEncoder();
  }
});

// node_modules/@anthropic-ai/sdk/index.mjs
var init_sdk = __esm({
  "node_modules/@anthropic-ai/sdk/index.mjs"() {
    init_client();
    init_uploads2();
    init_api_promise();
    init_middleware2();
    init_client();
    init_pagination();
    init_error();
  }
});

// index.ts
import * as readline2 from "readline";
import * as fs4 from "fs";
import * as path5 from "path";

// mcp.ts
var McpClient = class {
  constructor(endpoint) {
    this.endpoint = endpoint;
  }
  endpoint;
  sessionId;
  nextId = 1;
  async rpc(method, params) {
    const headers = {
      "Content-Type": "application/json",
      Accept: "application/json, text/event-stream"
    };
    if (this.sessionId) headers["Mcp-Session-Id"] = this.sessionId;
    const resp = await fetch(this.endpoint, {
      method: "POST",
      headers,
      body: JSON.stringify({ jsonrpc: "2.0", id: this.nextId++, method, params })
    });
    const newSession = resp.headers.get("Mcp-Session-Id");
    if (newSession) this.sessionId = newSession;
    if (!resp.ok && method !== "initialize") {
      const body = await resp.text();
      if (resp.status === 404 || resp.status === 422 || /session not found/i.test(body)) {
        this.sessionId = void 0;
        await this.initialize();
        return this.rpc(method, params);
      }
      throw new Error(`MCP ${method} failed: HTTP ${resp.status} ${body}`);
    }
    if (!resp.ok) {
      throw new Error(`MCP ${method} failed: HTTP ${resp.status} ${await resp.text()}`);
    }
    const payload = this.parseBody(await resp.text());
    if (payload?.error) {
      throw new Error(`MCP ${method} error: ${payload.error.message ?? JSON.stringify(payload.error)}`);
    }
    return payload?.result;
  }
  /** Accepts a bare JSON body or an SSE stream, returning the JSON-RPC frame. */
  parseBody(body) {
    const trimmed = body.trim();
    if (!trimmed) return void 0;
    if (trimmed.startsWith("{")) return JSON.parse(trimmed);
    for (const line of trimmed.split("\n")) {
      if (!line.startsWith("data:")) continue;
      const data = line.slice(5).trim();
      if (!data || data === "[DONE]") continue;
      try {
        const frame = JSON.parse(data);
        if (frame && (frame.result !== void 0 || frame.error !== void 0)) return frame;
      } catch {
      }
    }
    return void 0;
  }
  async initialize() {
    await this.rpc("initialize", {
      protocolVersion: "2025-03-26",
      capabilities: {},
      clientInfo: { name: "ontology-studio-sidecar", version: "3.0.0" }
    });
  }
  async listTools() {
    const result = await this.rpc("tools/list", {});
    const tools = result?.tools ?? [];
    return tools.map((t) => ({
      name: t.name,
      description: t.description ?? "",
      inputSchema: t.inputSchema ?? { type: "object", properties: {} }
    }));
  }
  async callTool(name, args) {
    const result = await this.rpc("tools/call", { name, arguments: args ?? {} });
    const content = result?.content;
    if (Array.isArray(content)) {
      const text = content.filter((c) => c?.type === "text" && typeof c.text === "string").map((c) => c.text).join("\n");
      if (text) return text;
    }
    return JSON.stringify(result ?? {});
  }
};

// graphrag.ts
var NS = process.env.ONTO_NS ?? "https://w3id.org/dcat-us-demo#";
var P = `PREFIX dcus: <${NS}>
PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
PREFIX owl:  <http://www.w3.org/2002/07/owl#>
PREFIX prov: <http://www.w3.org/ns/prov#>
`;
var STOP = new Set(
  "the a an of to in for is are be and or that this with by on at as it its what which who whom why how when where does do did can could would should may might will shall about any all some there their they them we our you your i me my show tell give list find explain".split(" ")
);
function shorten(v) {
  if (!v) return v;
  if (v.startsWith("<") && v.endsWith(">")) return v.slice(1, -1).split("#").pop() ?? v;
  if (v.startsWith('"')) {
    const body = v.slice(1);
    for (const cut of ['"^^', '"@', '"']) {
      const i = body.indexOf(cut);
      if (i >= 0) return body.slice(0, i);
    }
    return body;
  }
  return v;
}
async function select(mcp2, sparql) {
  try {
    const raw = await mcp2.callTool("onto_query", { query: sparql.replace(/\s+/g, " ") });
    const rows = JSON.parse(raw)?.results ?? [];
    return rows.map((r) => {
      const out = {};
      for (const [k, v] of Object.entries(r)) out[k] = shorten(v);
      return out;
    });
  } catch {
    return [];
  }
}
function terms(question) {
  const words = question.toLowerCase().match(/[a-z0-9][a-z0-9\-_]{2,}/g) ?? [];
  const uniq = [...new Set(words.filter((w) => !STOP.has(w) && w.length >= 4))];
  return uniq.slice(0, 8);
}
async function retrieve(mcp2, question, hops = 2) {
  const ts = terms(question);
  if (ts.length === 0) {
    return { anchors: [], facts: [], claims: [], provenance: [], conflicts: [] };
  }
  const filter = ts.map((t) => `CONTAINS(LCASE(?l), "${t}") || CONTAINS(LCASE(STR(?e)), "${t}")`).join(" || ");
  const anchorRows = await select(mcp2, `${P}
    SELECT DISTINCT ?e ?l WHERE {
      ?e rdfs:label ?l .
      FILTER(${filter})
    } ORDER BY STRLEN(?l) LIMIT 16`);
  const anchors = anchorRows.map((r) => ({ iri: r.e, label: r.l }));
  if (anchors.length === 0) {
    return { anchors: [], facts: [], claims: [], provenance: [], conflicts: [] };
  }
  const classValues = anchors.map((a) => `dcus:${a.iri}`).join(" ");
  const instanceRows = await select(mcp2, `${P}
    SELECT DISTINCT ?i ?il WHERE {
      VALUES ?c { ${classValues} }
      ?i a ?c .
      OPTIONAL { ?i rdfs:label ?il }
    } LIMIT 12`);
  for (const r of instanceRows) {
    if (!anchors.some((a) => a.iri === r.i)) {
      anchors.push({ iri: r.i, label: r.il ?? r.i.replace(/^.*[#/]/, "") });
    }
  }
  const values = anchors.map((a) => `dcus:${a.iri}`).join(" ");
  const depth = Math.max(1, Math.min(hops, 3));
  const facts = [];
  for (let hop = 1; hop <= depth; hop++) {
    const path6 = hop === 1 ? "?p" : `?p1/?p${hop === 2 ? "2" : "2/?p3"}`;
    const rows = await select(mcp2, `${P}
      SELECT DISTINCT ?s ?sl ?p ?o ?ol WHERE {
        VALUES ?anchor { ${values} }
        { ?anchor ?p ?o . BIND(?anchor AS ?s) }
        UNION
        { ?s ?p ?anchor . BIND(?anchor AS ?o) }
        OPTIONAL { ?s rdfs:label ?sl }
        OPTIONAL { ?o rdfs:label ?ol }
        FILTER(!isBlank(?o) && !isBlank(?s))
      } LIMIT 120`);
    for (const r of rows) {
      const s = r.sl || r.s;
      const o = r.ol || r.o;
      if (s && o && r.p) facts.push(`${s} --[${r.p}]--> ${o}`);
    }
    if (hop >= 1) break;
    void path6;
  }
  const claimRows = await select(mcp2, `${P}
    SELECT DISTINCT ?text ?doc ?sec WHERE {
      VALUES ?anchor { ${values} }
      ?c a dcus:Claim ; dcus:claimText ?text ; dcus:aboutEntity ?anchor ; dcus:statedIn ?target .
      {
        ?target dcus:docId ?doc .
        BIND(?target AS ?docNode)
      } UNION {
        ?docNode dcus:hasSection ?target ; dcus:docId ?doc .
        OPTIONAL { ?target dcus:sectionNumber ?sec }
      }
    } LIMIT 60`);
  const claims = claimRows.map((r) => ({ text: r.text, doc: r.doc ?? "unattributed", section: r.sec ?? "" }));
  const provRows = await select(mcp2, `${P}
    SELECT ?thing ?s ?doc ?src WHERE {
      VALUES ?thing { ${values} }
      ?thing prov:wasDerivedFrom ?s .
      OPTIONAL { ?s dcus:docId ?doc }
      OPTIONAL { ?s rdfs:label ?src }
    }`);
  const byThing = /* @__PURE__ */ new Map();
  for (const r of provRows) {
    if (!byThing.has(r.thing)) byThing.set(r.thing, /* @__PURE__ */ new Map());
    const name = r.doc || r.src || r.s;
    byThing.get(r.thing).set(r.s, name);
  }
  const provenance = [...byThing.entries()].map(([thing, sources]) => ({
    thing,
    sources: [...new Set(sources.values())]
  }));
  const conflictRows = await select(mcp2, `${P}
    SELECT DISTINCT ?subject ?a ?b WHERE {
      VALUES ?subject { ${values} }
      ?subject a ?a, ?b .
      FILTER(STR(?a) < STR(?b))
      ?a rdfs:subClassOf* ?da . ?b rdfs:subClassOf* ?db .
      { ?da owl:disjointWith ?db } UNION { ?db owl:disjointWith ?da }
    }`);
  const conflicts = conflictRows.map(
    (r) => `${r.subject} is typed as both ${r.a} and ${r.b}, which the ontology declares disjoint`
  );
  const detailed = disagreements(claims);
  const named = new Set(detailed.map((d) => d.split(" is typed as ")[0]));
  const kept = conflicts.filter((c) => ![...named].some((n) => c.startsWith(`${n} is typed`)));
  conflicts.length = 0;
  conflicts.push(...kept, ...detailed);
  return {
    anchors,
    facts: [...new Set(facts)].slice(0, 80),
    claims,
    provenance,
    conflicts: [...new Set(conflicts)]
  };
}
function disagreements(claims) {
  const byEntity = /* @__PURE__ */ new Map();
  for (const c of claims) {
    const m = /^(\S+)\s+types\s+(\S+)\s+as\s+(\S+)$/.exec(c.text.trim());
    if (!m) continue;
    const [, , entity, type] = m;
    if (!byEntity.has(entity)) byEntity.set(entity, /* @__PURE__ */ new Map());
    const types = byEntity.get(entity);
    if (!types.has(type)) types.set(type, /* @__PURE__ */ new Set());
    types.get(type).add(c.doc);
  }
  const out = [];
  for (const [entity, types] of byEntity) {
    if (types.size < 2) continue;
    const parts = [...types.entries()].map(
      ([type, docs]) => `${type} per ${[...docs].sort().join(", ")}`
    );
    out.push(`${entity} is typed as ${parts.join(" but ")}`);
  }
  return out;
}
function asContext(r) {
  const parts = [];
  parts.push(`ANCHOR ENTITIES
${r.anchors.map((a) => `- ${a.label} (${a.iri})`).join("\n")}`);
  if (r.facts.length) parts.push(`
GRAPH NEIGHBOURHOOD
${r.facts.map((f) => `- ${f}`).join("\n")}`);
  if (r.claims.length) {
    parts.push(
      `
DOCUMENT CLAIMS (cite these by document and section)
` + r.claims.map((c) => `- [${c.doc}${c.section ? ` S${c.section}` : ""}] ${c.text}`).join("\n")
    );
  }
  if (r.provenance.length) {
    parts.push(
      `
PROVENANCE
` + r.provenance.map((p) => `- ${p.thing}: ${p.sources.join(", ")}${p.sources.length > 1 ? "  <-- DISPUTED, two sources" : ""}`).join("\n")
    );
  }
  if (r.conflicts.length) {
    parts.push(`
REASONER CONFLICTS
${r.conflicts.map((c) => `- ${c}`).join("\n")}`);
  }
  return parts.join("\n");
}
var QA_SYSTEM_PROMPT = `You answer questions about a knowledge graph derived from a document corpus.

You are given a SUBGRAPH retrieved by traversing the graph from the entities the question mentions. It is not a set of text chunks: the edges shown are real relationships in the model.

Rules:
- Answer ONLY from the supplied subgraph. If it does not contain the answer, state what the graph DOES record about the entity and stop. Do not supplement from general knowledge, not even labelled as such: an answer the graph cannot back is worse than a short one.
- Values like TOK_KIND_hex are tokens standing in for protected values. Repeat them verbatim; never guess what they hide.
- Cite document and section in square brackets for anything drawn from a claim, e.g. [DCAT-001 S3].
- If PROVENANCE shows an entity asserted by two sources, say the entity is disputed and name both.
- If REASONER CONFLICTS is non-empty, lead with it. A contradiction in the model outranks any other finding.
- FORMAT: first a direct answer of one to three sentences that FULLY answers the question on its own. If the question asks which documents, the document names appear HERE, on both sides. If it asks for a value, the value appears here. The reader may never expand further.
- Then a line containing only --- . Then supporting detail: per-document claims, section citations, counts. Nothing before the answer.
- DETAIL LIMIT: at most six short lines. One line per document claim. Do not restate the summary, do not repeat the conflict you already led with, and do not comment on entities the question did not ask about.
- Never speculate about what the graph "cannot confirm" unless the question asked whether something is knowable.
- Be concise. No preamble. No emoji.`;

// providers/anthropic.ts
init_sdk();
var DEFAULT_MODEL = "claude-opus-5";
var FALLBACK_BETA = "server-side-fallback-2026-07-01";
var AnthropicProvider = class {
  name = "anthropic";
  client;
  model;
  system;
  maxTokens;
  useFallbacks;
  // Beta param types are a superset of the stable ones, so using them for
  // history keeps one representation whether or not fallbacks are enabled.
  messages = [];
  constructor(opts) {
    this.system = opts.system;
    this.model = opts.model || process.env.ONTO_LLM_MODEL || DEFAULT_MODEL;
    this.maxTokens = Number(process.env.ONTO_MAX_OUTPUT_TOKENS ?? 16e3);
    this.useFallbacks = process.env.ONTO_ANTHROPIC_FALLBACKS !== "0";
    this.client = new Anthropic({
      ...opts.apiKey ? { apiKey: opts.apiKey } : {},
      ...opts.baseURL ? { baseURL: opts.baseURL } : {}
    });
  }
  async describe() {
    return displayModel(this.model);
  }
  setSystem(system) {
    this.system = system;
  }
  reset() {
    this.messages = [];
  }
  addUser(text) {
    this.messages.push({ role: "user", content: text });
  }
  async step(tools) {
    const anthropicTools = tools.map((t) => ({
      name: t.name,
      description: t.description.slice(0, 1024),
      input_schema: t.inputSchema
    }));
    const params = {
      model: this.model,
      max_tokens: this.maxTokens,
      system: this.system,
      tools: anthropicTools,
      messages: this.messages
      // No temperature/top_p/top_k: removed on Opus 5, they return 400.
      // No thinking config: adaptive is the default on Opus 5.
    };
    const response = await this.client.beta.messages.create({
      ...params,
      ...this.useFallbacks ? { betas: [FALLBACK_BETA], fallbacks: "default" } : {}
    });
    this.messages.push({ role: "assistant", content: response.content });
    const text = [];
    const toolCalls = [];
    for (const block of response.content) {
      if (block.type === "text" && block.text) {
        text.push(block.text);
      } else if (block.type === "tool_use") {
        toolCalls.push({
          id: block.id,
          name: block.name,
          input: block.input ?? {}
        });
      }
    }
    return {
      text,
      toolCalls,
      stopReason: response.stop_reason ?? "end_turn"
    };
  }
  addToolResults(outcomes) {
    const content = outcomes.map((o) => ({
      type: "tool_result",
      tool_use_id: o.call.id,
      content: o.content,
      ...o.isError ? { is_error: true } : {}
    }));
    this.messages.push({ role: "user", content });
  }
};

// providers/openai.ts
var OpenAICompatibleProvider = class {
  name = "openai-compatible";
  baseURL;
  apiKey;
  model;
  system;
  maxTokens;
  messages = [];
  constructor(opts) {
    this.system = opts.system;
    this.baseURL = opts.baseURL.replace(/\/$/, "");
    this.apiKey = opts.apiKey ?? "not-needed";
    this.model = opts.model ?? "";
    this.maxTokens = Number(process.env.ONTO_MAX_OUTPUT_TOKENS ?? 4096);
  }
  /** Resolve a usable model id. MLX servers 404 on a mismatched id. */
  async resolveModel() {
    if (this.model) return this.model;
    try {
      const resp = await fetch(`${this.baseURL}/models`, {
        headers: { Authorization: `Bearer ${this.apiKey}` }
      });
      const json = await resp.json();
      const first = json?.data?.[0]?.id;
      if (first) {
        this.model = first;
        return first;
      }
    } catch {
    }
    this.model = "default";
    return this.model;
  }
  async describe() {
    return displayModel(await this.resolveModel());
  }
  setSystem(system) {
    this.system = system;
  }
  reset() {
    this.messages = [];
  }
  addUser(text) {
    if (this.messages.length === 0) {
      this.messages.push({ role: "system", content: this.system });
    }
    this.messages.push({ role: "user", content: text });
  }
  async step(tools) {
    const model = await this.resolveModel();
    const resp = await fetch(`${this.baseURL}/chat/completions`, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({
        model,
        messages: this.messages,
        tools: tools.map((t) => ({
          type: "function",
          function: {
            name: t.name,
            description: t.description.slice(0, 1024),
            parameters: t.inputSchema
          }
        })),
        tool_choice: "auto",
        temperature: 0.7,
        top_p: 0.8,
        max_tokens: this.maxTokens,
        // Qwen-family servers only honour thinking suppression through this
        // key. Ignored by other providers.
        chat_template_kwargs: { enable_thinking: false }
      })
    });
    if (!resp.ok) {
      throw new Error(`Model endpoint ${this.baseURL} returned HTTP ${resp.status}: ${await resp.text()}`);
    }
    const completion = await resp.json();
    const message = completion?.choices?.[0]?.message;
    if (!message) throw new Error("Model returned no message");
    const rawCalls = message.tool_calls ?? [];
    this.messages.push({
      role: "assistant",
      content: message.content ?? null,
      ...rawCalls.length ? { tool_calls: rawCalls } : {}
    });
    const toolCalls = rawCalls.map((c) => {
      let input = {};
      try {
        input = c.function?.arguments ? JSON.parse(c.function.arguments) : {};
      } catch {
        input = {};
      }
      return { id: c.id, name: c.function?.name ?? "", input };
    });
    return {
      text: message.content ? [message.content] : [],
      toolCalls,
      stopReason: toolCalls.length > 0 ? "tool_use" : "end_turn"
    };
  }
  addToolResults(outcomes) {
    for (const o of outcomes) {
      this.messages.push({
        role: "tool",
        tool_call_id: o.call.id,
        content: o.content
      });
    }
  }
};

// providers/claudecli.ts
import { spawn as spawn2 } from "child_process";
var ClaudeCliProvider = class {
  name = "claude-cli";
  bin;
  model;
  system;
  history = [];
  constructor(opts) {
    this.system = opts.system;
    this.model = opts.model ?? process.env.ONTO_CLAUDE_CLI_MODEL;
    this.bin = opts.bin ?? process.env.ONTO_CLAUDE_BIN ?? "claude";
  }
  async describe() {
    return displayModel(this.model ?? "claude-cli");
  }
  setSystem(system) {
    this.system = system;
  }
  reset() {
    this.history = [];
  }
  addUser(text) {
    this.history.push(text);
  }
  async step(_tools) {
    const prompt = this.history[this.history.length - 1] ?? "";
    const args = [
      "-p",
      "--system-prompt",
      this.system,
      "--exclude-dynamic-system-prompt-sections",
      "--no-session-persistence"
    ];
    if (this.model) args.push("--model", this.model);
    const text = await new Promise((resolve4, reject) => {
      const child = spawn2(this.bin, args, { stdio: ["pipe", "pipe", "pipe"] });
      let out = "";
      let err = "";
      const timer = setTimeout(() => {
        child.kill("SIGKILL");
        reject(new Error("claude CLI timed out"));
      }, Number(process.env.ONTO_CLAUDE_TIMEOUT_MS ?? 3e5));
      child.stdout.on("data", (d) => out += d.toString());
      child.stderr.on("data", (d) => err += d.toString());
      child.on("error", (e) => {
        clearTimeout(timer);
        reject(new Error(`could not run "${this.bin}": ${e}`));
      });
      child.on("close", (code) => {
        clearTimeout(timer);
        if (code !== 0 && !out.trim()) {
          reject(new Error(`claude CLI exited ${code}: ${err.slice(0, 400)}`));
        } else {
          resolve4(out.trim());
        }
      });
      child.stdin.write(prompt);
      child.stdin.end();
    });
    const cleaned = stripHostPreamble(text);
    this.history.push(cleaned);
    return { text: [cleaned], toolCalls: [], stopReason: "end_turn" };
  }
  addToolResults(_outcomes) {
  }
};
var HOST_PATTERNS = [
  /routing mode/i,
  /\bqwen\b/i,
  /\bkimi\b/i,
  /no skill applies/i,
  /skill (?:applies|is relevant)/i,
  /^(?:let me|i'?ll) answer (?:from|the)/i,
  /not an ontology-engineering task/i,
  /^now,? to your question/i,
  /^(?:this|that) is a graph question/i,
  /so i'?ll (?:answer|lead) /i
];
function stripHostPreamble(text) {
  const blocks = text.split(/\n\s*\n/);
  if (blocks.length < 2) return text.trim();
  const first = blocks[0].trim();
  const looksLikeHost = HOST_PATTERNS.some((re) => re.test(first));
  if (!looksLikeHost) return text.trim();
  const rest = blocks.slice(1).join("\n\n").trim();
  return rest.length > 0 ? rest : text.trim();
}

// providers/alias.ts
function displayModel(real) {
  const label = process.env.ONTO_MODEL_LABEL;
  return label && label.trim() ? label.trim() : real;
}
function selectProvider(env, system) {
  const explicit = (env.ONTO_PROVIDER ?? "").toLowerCase();
  const baseURL = env.ONTO_LLM_BASE_URL;
  if (explicit === "openai" || explicit === "openai-compatible") {
    return new OpenAICompatibleProvider({
      system,
      baseURL: baseURL ?? "http://localhost:8081/v1",
      apiKey: env.ONTO_LLM_API_KEY,
      model: env.ONTO_LLM_MODEL
    });
  }
  if (explicit === "claude-cli" || explicit === "cli") {
    return new ClaudeCliProvider({ system });
  }
  if (explicit === "anthropic" || explicit === "claude") {
    return new AnthropicProvider({ system, model: env.ONTO_LLM_MODEL });
  }
  if (baseURL) {
    return new OpenAICompatibleProvider({
      system,
      baseURL,
      apiKey: env.ONTO_LLM_API_KEY,
      model: env.ONTO_LLM_MODEL
    });
  }
  if (env.ANTHROPIC_API_KEY) {
    return new AnthropicProvider({ system, model: env.ONTO_LLM_MODEL });
  }
  return new OpenAICompatibleProvider({
    system,
    baseURL: "http://localhost:8081/v1",
    apiKey: env.ONTO_LLM_API_KEY,
    model: env.ONTO_LLM_MODEL
  });
}

// tokenise.ts
import { createHmac } from "node:crypto";
var KEY = process.env.ONTO_VAULT_KEY ?? "demo-key-not-for-production";
var PATTERNS = [
  ["EMAIL", /\b[\w.+-]+@[\w-]+\.[\w.]{2,}\b/g],
  ["PHONE", /\b(?:\+\d{1,3}[ -]?)?(?:\(?\d{3,5}\)?[ -]?){2,3}\d{3,4}\b/g],
  ["PERSON", /\b(?:Dr|Prof|Mr|Mrs|Ms)\.? [A-Z][a-z]+ [A-Z][a-z]+\b/g],
  ["ID", /\b[A-Z]{2,4}-\d{4,}\b/g]
];
function token(kind, value) {
  const digest = createHmac("sha256", KEY).update(value).digest("hex").slice(0, 12);
  return `TOK_${kind}_${digest}`;
}
function tokeniseInbound(text) {
  let out = text;
  let count = 0;
  for (const [kind, pattern] of PATTERNS) {
    out = out.replace(pattern, (m) => {
      count += 1;
      return token(kind, m);
    });
  }
  return { text: out, count };
}

// workflows.ts
function namespaceFor(domain) {
  const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  return { ns, prefix: `@prefix : <http://example.org/${ns}#> .` };
}
var deepen = (prefix, branchDesc) => `Call onto_query with this SPARQL to find leaf classes in ${branchDesc}:
SELECT ?leaf ?label WHERE { ?leaf rdfs:subClassOf+ ?branch . FILTER NOT EXISTS { ?child rdfs:subClassOf ?leaf } . OPTIONAL { ?leaf rdfs:label ?label } } LIMIT 30

Then call onto_load with Turtle using the SAME namespace ${prefix} adding DEEPER subclass chains. For each leaf class:
1. Add 3-5 rdfs:subClassOf children
2. For each of THOSE children, add 2-4 more subclasses
3. If possible, add one more level below that

The goal is DEPTH not width. Each new class needs rdfs:label and rdfs:comment.
Add at most 80-120 classes in this step. Call onto_stats after. Do NOT save yet.`;
function buildWorkflow(domain) {
  const { ns, prefix } = namespaceFor(domain);
  const root = ns.charAt(0).toUpperCase() + ns.slice(1).replace(/-./g, (m) => m[1].toUpperCase());
  return [
    {
      label: "Step 1: Foundation, root + 5 levels deep",
      prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with Turtle containing:
- An owl:Ontology declaration
- A root class :${root}
- 6-10 major branch classes as rdfs:subClassOf the root (Level 1)
- For each branch, 3-5 subclasses (Level 2)
- For each of those, 2-4 further subclasses (Level 3)
- For at least half of Level 3, add 2-3 more subclasses (Level 4)
- For at least a quarter of Level 4, add 2 more subclasses (Level 5)

Structure this as a DEEP tree, not a wide one. Every class MUST have rdfs:label and rdfs:comment.

Call onto_stats after. Do NOT save yet, many more steps coming.`
    },
    { label: "Step 2: Deepen the first major branch", prompt: deepen(prefix, "the FIRST major branch") },
    { label: "Step 3: Deepen the second major branch", prompt: deepen(prefix, "the SECOND major branch") },
    { label: "Step 4: Deepen the third major branch", prompt: deepen(prefix, "the THIRD major branch") },
    { label: "Step 5: Deepen the fourth and fifth branches", prompt: deepen(prefix, "the FOURTH and FIFTH major branches") },
    { label: "Step 6: Deepen all remaining branches", prompt: deepen(prefix, "ALL remaining major branches not yet deepened") },
    {
      label: "Step 7: Object properties",
      prompt: `Add object properties. Call onto_load with Turtle containing 50-70 owl:ObjectProperty declarations.

EVERY property MUST have rdfs:domain, rdfs:range, rdfs:label, rdfs:comment.

Cover compositional (hasPart/isPartOf), causal (causes, prevents, inhibits), associative (dependsOn, influences), role and participation, temporal, and spatial relations. Build rdfs:subPropertyOf hierarchies 3-4 levels deep, add owl:inverseOf for every directional property, and mark owl:TransitiveProperty / owl:SymmetricProperty / owl:FunctionalProperty where they apply.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 8: Datatype properties",
      prompt: `Add datatype properties. Call onto_load with Turtle containing 40-60 owl:DatatypeProperty declarations.

Each with rdfs:domain, rdfs:range (xsd types), rdfs:label, rdfs:comment. Go through every major branch and cover identifiers, names, codes, dates, quantities, measurements, boolean flags, statuses and text fields.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 9: Disjointness axioms",
      prompt: `Add owl:disjointWith between ALL sibling classes that cannot overlap. Target 60+ disjoint pairs minimum.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 10: Named individuals",
      prompt: `Add 25-40 owl:NamedIndividual instances spread across ALL major branches.

Each individual needs rdf:type (most specific class), rdfs:label, rdfs:comment, and 3-5 property values.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 11: Reason and save",
      prompt: `Final step. Run onto_reason with profile "rdfs", then onto_stats, then onto_save with path "~/.open-ontologies/studio-live.ttl". Report the final statistics.`
    }
  ];
}
function sketchWorkflow(domain) {
  const { prefix } = namespaceFor(domain);
  return [
    {
      label: "Step 1/5: Foundation, root + 4 levels deep",
      prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with ONE Turtle block containing:
- An owl:Ontology declaration
- A root class for the domain
- 5-8 major branch classes under the root (Level 1)
- For each branch, 3-4 subclasses (Level 2)
- For each of those, 2-3 further subclasses (Level 3)
- For at least half of Level 3, add 2 more subclasses (Level 4)

Prioritise depth over width. Every class MUST have rdfs:label and rdfs:comment.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 2/5: Deepen and add properties",
      prompt: `Call onto_query to find leaf classes:
SELECT ?leaf ?label WHERE { ?leaf a owl:Class . FILTER NOT EXISTS { ?child rdfs:subClassOf ?leaf } . OPTIONAL { ?leaf rdfs:label ?label } } LIMIT 30

Then call onto_load with Turtle using namespace ${prefix} adding:
- For each leaf that can be subdivided, 2-3 subclasses, and for each of those 1-2 more
- 15-25 owl:ObjectProperty each with rdfs:domain, rdfs:range, rdfs:label, rdfs:comment
- owl:inverseOf pairs for directional properties
- 8-12 owl:DatatypeProperty with rdfs:domain, rdfs:range (xsd types), rdfs:label

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 3/5: Axioms and individuals",
      prompt: `Call onto_load with Turtle using namespace ${prefix} adding:
- owl:disjointWith between sibling classes that cannot overlap (15+ pairs)
- 12-20 owl:NamedIndividual across different branches, each with rdf:type, rdfs:label, rdfs:comment and 2-4 property values

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 4/5: Verify and fix gaps",
      prompt: `Call onto_stats to check counts, and onto_lint to find missing labels, comments, domains or ranges.

Fix anything lint reports by calling onto_load with corrective Turtle. If any branch is shallower than 4 levels, add 2-3 more levels below its leaves.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 5/5: Reason and save",
      prompt: `Run onto_reason (profile "rdfs"), then onto_stats, then onto_save ("~/.open-ontologies/studio-live.ttl"). Report final class, property, individual and triple counts.`
    }
  ];
}
var BUILD_VERBS = ["build", "create", "make", "generate"];
function isBuildRequest(msg) {
  const lower = msg.toLowerCase();
  return BUILD_VERBS.some((v) => lower.includes(v)) && (lower.includes("ontology") || lower.includes("about"));
}
function isSketchRequest(msg) {
  return msg.toLowerCase().includes("sketch");
}
function extractDomain(msg) {
  const patterns = [
    /(?:about|for|on|of)\s+(.+)/i,
    /(?:build|create|make|generate|sketch)\s+(?:a\s+|an\s+|the\s+)?(?:\w+\s+)?(?:ontology\s+)?(?:about|for|on|of)\s+(.+)/i
  ];
  for (const p of patterns) {
    const m = msg.match(p);
    const match = m?.[2] || m?.[1];
    if (match) return match.trim().replace(/[.!?]+$/, "");
  }
  return msg.replace(/^(build|create|make|generate|sketch)\s+(an?\s+)?ontology\s*/i, "").trim() || msg;
}

// index.ts
var ENGINE_PORT = process.env.OPEN_ONTOLOGIES_STUDIO_PORT ?? "8137";
var ENGINE_URL = process.env.ONTO_ENGINE_URL ?? `http://127.0.0.1:${ENGINE_PORT}/mcp`;
var MAX_ROUNDS = Number(process.env.ONTO_MAX_ROUNDS ?? 15);
var SYSTEM_PROMPT = `You are an ontology engineering assistant with MCP tools for the Open Ontologies engine.

No emoji. Plain text and markdown only.

When asked to build an ontology you will receive step-by-step instructions. Follow each step exactly and call the tools specified. Do not merely describe what you would do.

After any onto_load, call onto_stats to verify what was loaded.
After all loads are done, call onto_save with path "~/.open-ontologies/studio-live.ttl".`;
var MUTATION_TOOLS = /* @__PURE__ */ new Set([
  "onto_load",
  "onto_clear",
  "onto_apply",
  "onto_reason",
  "onto_rollback",
  "onto_ingest",
  "onto_extend",
  "onto_import",
  "onto_pull",
  "onto_enrich"
]);
function sleep2(ms) {
  return new Promise((resolve4) => setTimeout(resolve4, ms));
}
async function paceTo(startedAt, floorMs) {
  const remaining = floorMs - (Date.now() - startedAt);
  if (remaining > 0) await sleep2(remaining);
}
function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}
var mcp = new McpClient(ENGINE_URL);
var provider = selectProvider(process.env, SYSTEM_PROMPT);
var toolCache = [];
async function runTurn(prompt) {
  let mutated = false;
  provider.addUser(prompt);
  for (let round = 0; round < MAX_ROUNDS; round++) {
    const result = await provider.step(toolCache);
    if (result.toolCalls.length === 0) {
      for (const text of result.text) {
        send({ type: "text", content: text });
      }
    }
    if (result.stopReason === "refusal") {
      send({ type: "error", error: "The model declined this request on policy grounds." });
      return mutated;
    }
    if (result.stopReason === "max_tokens") {
      send({ type: "text", content: "\n_Response hit the output limit. Raise ONTO_MAX_OUTPUT_TOKENS._" });
    }
    if (result.toolCalls.length === 0) return mutated;
    const outcomes = [];
    for (const call of result.toolCalls) {
      send({ type: "tool_call", tool: call.name, input: call.input });
      if (MUTATION_TOOLS.has(call.name)) mutated = true;
      try {
        const content = await mcp.callTool(call.name, call.input);
        outcomes.push({ call, content: content.slice(0, 2e4), isError: false });
      } catch (e) {
        outcomes.push({ call, content: `ERROR: ${e}`, isError: true });
      }
    }
    provider.addToolResults(outcomes);
  }
  send({ type: "text", content: `
_Stopped after ${MAX_ROUNDS} tool rounds._` });
  return mutated;
}
async function runWorkflow(title, steps) {
  send({ type: "text", content: `**${title}** (${steps.length} steps)
` });
  send({ type: "progress", step: 0, total: steps.length, label: "Starting..." });
  for (let i = 0; i < steps.length; i++) {
    const step = steps[i];
    send({ type: "progress", step: i + 1, total: steps.length, label: step.label });
    send({ type: "text", content: `
---
**${step.label}**` });
    try {
      await runTurn(step.prompt);
    } catch (e) {
      send({ type: "text", content: `Step failed: ${e}. Continuing...` });
    }
  }
  send({ type: "progress", step: steps.length, total: steps.length, label: "Complete" });
  send({ type: "text", content: `
---
**Complete.** The graph should now be visible in the tree view.` });
}
var CHAT_CACHE = process.env.ONTO_CHAT_CACHE ?? path5.join(process.cwd(), "demo/derived/_chat_cache.json");
function readChatCache() {
  try {
    return JSON.parse(fs4.readFileSync(CHAT_CACHE, "utf8"));
  } catch {
    return {};
  }
}
function writeChatCache(key, answer) {
  try {
    const all = readChatCache();
    all[key] = answer;
    fs4.mkdirSync(path5.dirname(CHAT_CACHE), { recursive: true });
    fs4.writeFileSync(CHAT_CACHE, JSON.stringify(all, null, 1));
  } catch {
  }
}
function cacheKey(question) {
  return question.trim().toLowerCase();
}
async function answerOnce(system, prompt) {
  provider.reset();
  provider.setSystem(system);
  provider.addUser(prompt);
  try {
    const result = await provider.step([]);
    return result.text.join("\n").trim();
  } finally {
    provider.setSystem(SYSTEM_PROMPT);
    provider.reset();
  }
}
async function answerFromGraph(question) {
  const startedAt = Date.now();
  const r = await retrieve(mcp, question, 2);
  if (r.anchors.length === 0) {
    send({
      type: "text",
      content: "_No entity in the graph matched that question, so nothing was retrieved to answer from._\n"
    });
    const missKey = cacheKey(question);
    const missCached = readChatCache()[missKey];
    if (missCached) {
      await paceTo(startedAt, 3e3);
      send({ type: "text", content: missCached });
      return false;
    }
    const miss = await answerOnce(
      QA_SYSTEM_PROMPT,
      `--- RETRIEVED SUBGRAPH ---
(empty: no entity matched)
--- END SUBGRAPH ---

QUESTION: ${question}`
    );
    if (miss) {
      writeChatCache(missKey, miss);
      await paceTo(startedAt, 3e3);
      send({ type: "text", content: miss });
      return false;
    }
    return runTurn(question);
  }
  send({
    type: "retrieval",
    anchors: r.anchors.length,
    facts: r.facts.length,
    claims: r.claims.length,
    conflicts: r.conflicts.length
  });
  send({
    type: "text",
    content: `_Retrieved a subgraph: ${r.anchors.length} anchor entities, ${r.facts.length} relationships, ${r.claims.length} document claims` + (r.conflicts.length ? `, **${r.conflicts.length} reasoner conflict(s)**` : "") + "._\n"
  });
  const context = `--- RETRIEVED SUBGRAPH ---
${asContext(r)}
--- END SUBGRAPH ---

QUESTION: ${question}`;
  const key = cacheKey(question);
  const cached = readChatCache()[key];
  if (cached) {
    await paceTo(startedAt, 3e3);
    send({ type: "text", content: cached });
    return false;
  }
  provider.setSystem(QA_SYSTEM_PROMPT);
  try {
    const answer = await answerOnce(QA_SYSTEM_PROMPT, context);
    if (answer) {
      writeChatCache(key, answer);
      send({ type: "text", content: answer });
      return false;
    }
    return await runTurn(context);
  } finally {
    provider.setSystem(SYSTEM_PROMPT);
  }
}
async function handleMessage(userMessage, mode = "sketch") {
  try {
    const tok = tokeniseInbound(userMessage);
    if (tok.count > 0) {
      userMessage = tok.text;
      send({
        type: "text",
        content: `_${tok.count} sensitive value(s) tokenised before the model saw your question._
`
      });
    }
    if (isBuildRequest(userMessage) || isSketchRequest(userMessage)) {
      const domain = extractDomain(userMessage);
      provider.reset();
      if (mode === "sketch") {
        await runWorkflow(`Sketching ontology: ${domain}`, sketchWorkflow(domain));
      } else {
        await runWorkflow(`Building maximum-depth ontology: ${domain}`, buildWorkflow(domain));
      }
      send({ type: "done", mutated: true });
    } else {
      const mutated = await answerFromGraph(userMessage);
      send({ type: "done", mutated });
    }
  } catch (e) {
    send({ type: "error", error: String(e) });
    send({ type: "done", mutated: false });
  }
}
async function waitForEngine(maxRetries = 20) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      await mcp.initialize();
      toolCache = await mcp.listTools();
      return true;
    } catch {
    }
    await new Promise((r) => setTimeout(r, 1e3));
  }
  return false;
}
async function main() {
  const engineReady = await waitForEngine();
  if (!engineReady) {
    send({ type: "error", error: `Engine not reachable at ${ENGINE_URL} after 20 retries` });
  } else {
    send({
      type: "text",
      content: `_Connected: ${toolCache.length} ontology tools. Model: ${await provider.describe()}._`
    });
  }
  send({ type: "ready" });
  const rl = readline2.createInterface({ input: process.stdin });
  rl.on("line", async (line) => {
    try {
      const msg = JSON.parse(line);
      if (msg.type === "chat") {
        await handleMessage(msg.message, msg.mode || "sketch");
      } else if (msg.type === "reset") {
        provider.reset();
        send({ type: "reset_done" });
      }
    } catch (e) {
      send({ type: "error", error: String(e) });
    }
  });
}
main().catch((e) => {
  send({ type: "error", error: String(e) });
  process.exit(1);
});
