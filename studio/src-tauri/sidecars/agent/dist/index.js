// node_modules/@anthropic-ai/claude-agent-sdk/sdk.mjs
import { join as XV } from "path";
import { fileURLToPath as wE } from "url";
import { setMaxListeners as bV } from "events";
import { spawn as OU } from "child_process";
import { createInterface as DU } from "readline";
import { randomUUID as mq } from "crypto";
import { appendFile as lq, mkdir as cq } from "fs/promises";
import { join as S5 } from "path";
import { join as fq } from "path";
import { homedir as uq } from "os";
import * as u from "fs";
import { stat as oq, readdir as rq, readFile as T5, unlink as tq, rmdir as aq, rm as sq, mkdir as eq, rename as QU, open as XU } from "fs/promises";
import { dirname as u5, join as m5 } from "path";
import { cwd as JU } from "process";
import { realpathSync as x5 } from "fs";
import { randomUUID as i4 } from "crypto";
import { execFile as jU } from "child_process";
import { promisify as RU } from "util";
var LV = Object.create;
var { getPrototypeOf: FV, defineProperty: yQ, getOwnPropertyNames: NV } = Object;
var OV = Object.prototype.hasOwnProperty;
function DV(Q) {
  return this[Q];
}
var MV;
var wV;
var s7 = (Q, X, Y) => {
  var $ = Q != null && typeof Q === "object";
  if ($) {
    var J = X ? MV ??= /* @__PURE__ */ new WeakMap() : wV ??= /* @__PURE__ */ new WeakMap(), W = J.get(Q);
    if (W) return W;
  }
  Y = Q != null ? LV(FV(Q)) : {};
  let G = X || !Q || !Q.__esModule ? yQ(Y, "default", { value: Q, enumerable: true }) : Y;
  for (let H of NV(Q)) if (!OV.call(G, H)) yQ(G, H, { get: DV.bind(Q, H), enumerable: true });
  if ($) J.set(Q, G);
  return G;
};
var E = (Q, X) => () => (X || Q((X = { exports: {} }).exports, X), X.exports);
var AV = (Q) => Q;
function jV(Q, X) {
  this[Q] = AV.bind(null, X);
}
var gQ = (Q, X) => {
  for (var Y in X) yQ(Q, Y, { get: X[Y], enumerable: true, configurable: true, set: jV.bind(X, Y) });
};
var RV = Symbol.dispose || /* @__PURE__ */ Symbol.for("Symbol.dispose");
var IV = Symbol.asyncDispose || /* @__PURE__ */ Symbol.for("Symbol.asyncDispose");
var X0 = (Q, X, Y) => {
  if (X != null) {
    if (typeof X !== "object" && typeof X !== "function") throw TypeError('Object expected to be assigned to "using" declaration');
    var $;
    if (Y) $ = X[IV];
    if ($ === void 0) $ = X[RV];
    if (typeof $ !== "function") throw TypeError("Object not disposable");
    Q.push([Y, $, X]);
  } else if (Y) Q.push([Y]);
  return X;
};
var Y0 = (Q, X, Y) => {
  var $ = typeof SuppressedError === "function" ? SuppressedError : function(G, H, B, z) {
    return z = Error(B), z.name = "SuppressedError", z.error = G, z.suppressed = H, z;
  }, J = (G) => X = Y ? new $(G, X, "An error was suppressed during disposal") : (Y = true, G), W = (G) => {
    while (G = Q.pop()) try {
      var H = G[1] && G[1].call(G[2]);
      if (G[0]) return Promise.resolve(H).then(W, (B) => (J(B), W()));
    } catch (B) {
      J(B);
    }
    if (Y) throw X;
  };
  return W();
};
var M4 = E((H3) => {
  Object.defineProperty(H3, "__esModule", { value: true });
  H3.regexpCode = H3.getEsmExportName = H3.getProperty = H3.safeStringify = H3.stringify = H3.strConcat = H3.addCodeArg = H3.str = H3._ = H3.nil = H3._Code = H3.Name = H3.IDENTIFIER = H3._CodeOrName = void 0;
  class a8 {
  }
  H3._CodeOrName = a8;
  H3.IDENTIFIER = /^[a-z$_][a-z$_0-9]*$/i;
  class M9 extends a8 {
    constructor(Q) {
      super();
      if (!H3.IDENTIFIER.test(Q)) throw Error("CodeGen: name must be a valid identifier");
      this.str = Q;
    }
    toString() {
      return this.str;
    }
    emptyStr() {
      return false;
    }
    get names() {
      return { [this.str]: 1 };
    }
  }
  H3.Name = M9;
  class z1 extends a8 {
    constructor(Q) {
      super();
      this._items = typeof Q === "string" ? [Q] : Q;
    }
    toString() {
      return this.str;
    }
    emptyStr() {
      if (this._items.length > 1) return false;
      let Q = this._items[0];
      return Q === "" || Q === '""';
    }
    get str() {
      var Q;
      return (Q = this._str) !== null && Q !== void 0 ? Q : this._str = this._items.reduce((X, Y) => `${X}${Y}`, "");
    }
    get names() {
      var Q;
      return (Q = this._names) !== null && Q !== void 0 ? Q : this._names = this._items.reduce((X, Y) => {
        if (Y instanceof M9) X[Y.str] = (X[Y.str] || 0) + 1;
        return X;
      }, {});
    }
  }
  H3._Code = z1;
  H3.nil = new z1("");
  function W3(Q, ...X) {
    let Y = [Q[0]], $ = 0;
    while ($ < X.length) g$(Y, X[$]), Y.push(Q[++$]);
    return new z1(Y);
  }
  H3._ = W3;
  var y$ = new z1("+");
  function G3(Q, ...X) {
    let Y = [D4(Q[0])], $ = 0;
    while ($ < X.length) Y.push(y$), g$(Y, X[$]), Y.push(y$, D4(Q[++$]));
    return XM(Y), new z1(Y);
  }
  H3.str = G3;
  function g$(Q, X) {
    if (X instanceof z1) Q.push(...X._items);
    else if (X instanceof M9) Q.push(X);
    else Q.push(JM(X));
  }
  H3.addCodeArg = g$;
  function XM(Q) {
    let X = 1;
    while (X < Q.length - 1) {
      if (Q[X] === y$) {
        let Y = YM(Q[X - 1], Q[X + 1]);
        if (Y !== void 0) {
          Q.splice(X - 1, 3, Y);
          continue;
        }
        Q[X++] = "+";
      }
      X++;
    }
  }
  function YM(Q, X) {
    if (X === '""') return Q;
    if (Q === '""') return X;
    if (typeof Q == "string") {
      if (X instanceof M9 || Q[Q.length - 1] !== '"') return;
      if (typeof X != "string") return `${Q.slice(0, -1)}${X}"`;
      if (X[0] === '"') return Q.slice(0, -1) + X.slice(1);
      return;
    }
    if (typeof X == "string" && X[0] === '"' && !(Q instanceof M9)) return `"${Q}${X.slice(1)}`;
    return;
  }
  function $M(Q, X) {
    return X.emptyStr() ? Q : Q.emptyStr() ? X : G3`${Q}${X}`;
  }
  H3.strConcat = $M;
  function JM(Q) {
    return typeof Q == "number" || typeof Q == "boolean" || Q === null ? Q : D4(Array.isArray(Q) ? Q.join(",") : Q);
  }
  function WM(Q) {
    return new z1(D4(Q));
  }
  H3.stringify = WM;
  function D4(Q) {
    return JSON.stringify(Q).replace(/\u2028/g, "\\u2028").replace(/\u2029/g, "\\u2029");
  }
  H3.safeStringify = D4;
  function GM(Q) {
    return typeof Q == "string" && H3.IDENTIFIER.test(Q) ? new z1(`.${Q}`) : W3`[${Q}]`;
  }
  H3.getProperty = GM;
  function HM(Q) {
    if (typeof Q == "string" && H3.IDENTIFIER.test(Q)) return new z1(`${Q}`);
    throw Error(`CodeGen: invalid export name: ${Q}, use explicit $id name mapping`);
  }
  H3.getEsmExportName = HM;
  function BM(Q) {
    return new z1(Q.toString());
  }
  H3.regexpCode = BM;
});
var m$ = E((V3) => {
  Object.defineProperty(V3, "__esModule", { value: true });
  V3.ValueScope = V3.ValueScopeName = V3.Scope = V3.varKinds = V3.UsedValueState = void 0;
  var g0 = M4();
  class z3 extends Error {
    constructor(Q) {
      super(`CodeGen: "code" for ${Q} not defined`);
      this.value = Q.value;
    }
  }
  var e8;
  (function(Q) {
    Q[Q.Started = 0] = "Started", Q[Q.Completed = 1] = "Completed";
  })(e8 || (V3.UsedValueState = e8 = {}));
  V3.varKinds = { const: new g0.Name("const"), let: new g0.Name("let"), var: new g0.Name("var") };
  class f$ {
    constructor({ prefixes: Q, parent: X } = {}) {
      this._names = {}, this._prefixes = Q, this._parent = X;
    }
    toName(Q) {
      return Q instanceof g0.Name ? Q : this.name(Q);
    }
    name(Q) {
      return new g0.Name(this._newName(Q));
    }
    _newName(Q) {
      let X = this._names[Q] || this._nameGroup(Q);
      return `${Q}${X.index++}`;
    }
    _nameGroup(Q) {
      var X, Y;
      if (((Y = (X = this._parent) === null || X === void 0 ? void 0 : X._prefixes) === null || Y === void 0 ? void 0 : Y.has(Q)) || this._prefixes && !this._prefixes.has(Q)) throw Error(`CodeGen: prefix "${Q}" is not allowed in this scope`);
      return this._names[Q] = { prefix: Q, index: 0 };
    }
  }
  V3.Scope = f$;
  class u$ extends g0.Name {
    constructor(Q, X) {
      super(X);
      this.prefix = Q;
    }
    setValue(Q, { property: X, itemIndex: Y }) {
      this.value = Q, this.scopePath = g0._`.${new g0.Name(X)}[${Y}]`;
    }
  }
  V3.ValueScopeName = u$;
  var AM = g0._`\n`;
  class K3 extends f$ {
    constructor(Q) {
      super(Q);
      this._values = {}, this._scope = Q.scope, this.opts = { ...Q, _n: Q.lines ? AM : g0.nil };
    }
    get() {
      return this._scope;
    }
    name(Q) {
      return new u$(Q, this._newName(Q));
    }
    value(Q, X) {
      var Y;
      if (X.ref === void 0) throw Error("CodeGen: ref must be passed in value");
      let $ = this.toName(Q), { prefix: J } = $, W = (Y = X.key) !== null && Y !== void 0 ? Y : X.ref, G = this._values[J];
      if (G) {
        let z = G.get(W);
        if (z) return z;
      } else G = this._values[J] = /* @__PURE__ */ new Map();
      G.set(W, $);
      let H = this._scope[J] || (this._scope[J] = []), B = H.length;
      return H[B] = X.ref, $.setValue(X, { property: J, itemIndex: B }), $;
    }
    getValue(Q, X) {
      let Y = this._values[Q];
      if (!Y) return;
      return Y.get(X);
    }
    scopeRefs(Q, X = this._values) {
      return this._reduceValues(X, (Y) => {
        if (Y.scopePath === void 0) throw Error(`CodeGen: name "${Y}" has no value`);
        return g0._`${Q}${Y.scopePath}`;
      });
    }
    scopeCode(Q = this._values, X, Y) {
      return this._reduceValues(Q, ($) => {
        if ($.value === void 0) throw Error(`CodeGen: name "${$}" has no value`);
        return $.value.code;
      }, X, Y);
    }
    _reduceValues(Q, X, Y = {}, $) {
      let J = g0.nil;
      for (let W in Q) {
        let G = Q[W];
        if (!G) continue;
        let H = Y[W] = Y[W] || /* @__PURE__ */ new Map();
        G.forEach((B) => {
          if (H.has(B)) return;
          H.set(B, e8.Started);
          let z = X(B);
          if (z) {
            let K = this.opts.es5 ? V3.varKinds.var : V3.varKinds.const;
            J = g0._`${J}${K} ${B} = ${z};${this.opts._n}`;
          } else if (z = $ === null || $ === void 0 ? void 0 : $(B)) J = g0._`${J}${z}${this.opts._n}`;
          else throw new z3(B);
          H.set(B, e8.Completed);
        });
      }
      return J;
    }
  }
  V3.ValueScope = K3;
});
var c = E((h0) => {
  Object.defineProperty(h0, "__esModule", { value: true });
  h0.or = h0.and = h0.not = h0.CodeGen = h0.operators = h0.varKinds = h0.ValueScopeName = h0.ValueScope = h0.Scope = h0.Name = h0.regexpCode = h0.stringify = h0.getProperty = h0.nil = h0.strConcat = h0.str = h0._ = void 0;
  var r = M4(), K1 = m$(), Q6 = M4();
  Object.defineProperty(h0, "_", { enumerable: true, get: function() {
    return Q6._;
  } });
  Object.defineProperty(h0, "str", { enumerable: true, get: function() {
    return Q6.str;
  } });
  Object.defineProperty(h0, "strConcat", { enumerable: true, get: function() {
    return Q6.strConcat;
  } });
  Object.defineProperty(h0, "nil", { enumerable: true, get: function() {
    return Q6.nil;
  } });
  Object.defineProperty(h0, "getProperty", { enumerable: true, get: function() {
    return Q6.getProperty;
  } });
  Object.defineProperty(h0, "stringify", { enumerable: true, get: function() {
    return Q6.stringify;
  } });
  Object.defineProperty(h0, "regexpCode", { enumerable: true, get: function() {
    return Q6.regexpCode;
  } });
  Object.defineProperty(h0, "Name", { enumerable: true, get: function() {
    return Q6.Name;
  } });
  var WQ = m$();
  Object.defineProperty(h0, "Scope", { enumerable: true, get: function() {
    return WQ.Scope;
  } });
  Object.defineProperty(h0, "ValueScope", { enumerable: true, get: function() {
    return WQ.ValueScope;
  } });
  Object.defineProperty(h0, "ValueScopeName", { enumerable: true, get: function() {
    return WQ.ValueScopeName;
  } });
  Object.defineProperty(h0, "varKinds", { enumerable: true, get: function() {
    return WQ.varKinds;
  } });
  h0.operators = { GT: new r._Code(">"), GTE: new r._Code(">="), LT: new r._Code("<"), LTE: new r._Code("<="), EQ: new r._Code("==="), NEQ: new r._Code("!=="), NOT: new r._Code("!"), OR: new r._Code("||"), AND: new r._Code("&&"), ADD: new r._Code("+") };
  class X6 {
    optimizeNodes() {
      return this;
    }
    optimizeNames(Q, X) {
      return this;
    }
  }
  class U3 extends X6 {
    constructor(Q, X, Y) {
      super();
      this.varKind = Q, this.name = X, this.rhs = Y;
    }
    render({ es5: Q, _n: X }) {
      let Y = Q ? K1.varKinds.var : this.varKind, $ = this.rhs === void 0 ? "" : ` = ${this.rhs}`;
      return `${Y} ${this.name}${$};` + X;
    }
    optimizeNames(Q, X) {
      if (!Q[this.name.str]) return;
      if (this.rhs) this.rhs = A9(this.rhs, Q, X);
      return this;
    }
    get names() {
      return this.rhs instanceof r._CodeOrName ? this.rhs.names : {};
    }
  }
  class p$ extends X6 {
    constructor(Q, X, Y) {
      super();
      this.lhs = Q, this.rhs = X, this.sideEffects = Y;
    }
    render({ _n: Q }) {
      return `${this.lhs} = ${this.rhs};` + Q;
    }
    optimizeNames(Q, X) {
      if (this.lhs instanceof r.Name && !Q[this.lhs.str] && !this.sideEffects) return;
      return this.rhs = A9(this.rhs, Q, X), this;
    }
    get names() {
      let Q = this.lhs instanceof r.Name ? {} : { ...this.lhs.names };
      return JQ(Q, this.rhs);
    }
  }
  class L3 extends p$ {
    constructor(Q, X, Y, $) {
      super(Q, Y, $);
      this.op = X;
    }
    render({ _n: Q }) {
      return `${this.lhs} ${this.op}= ${this.rhs};` + Q;
    }
  }
  class F3 extends X6 {
    constructor(Q) {
      super();
      this.label = Q, this.names = {};
    }
    render({ _n: Q }) {
      return `${this.label}:` + Q;
    }
  }
  class N3 extends X6 {
    constructor(Q) {
      super();
      this.label = Q, this.names = {};
    }
    render({ _n: Q }) {
      return `break${this.label ? ` ${this.label}` : ""};` + Q;
    }
  }
  class O3 extends X6 {
    constructor(Q) {
      super();
      this.error = Q;
    }
    render({ _n: Q }) {
      return `throw ${this.error};` + Q;
    }
    get names() {
      return this.error.names;
    }
  }
  class D3 extends X6 {
    constructor(Q) {
      super();
      this.code = Q;
    }
    render({ _n: Q }) {
      return `${this.code};` + Q;
    }
    optimizeNodes() {
      return `${this.code}` ? this : void 0;
    }
    optimizeNames(Q, X) {
      return this.code = A9(this.code, Q, X), this;
    }
    get names() {
      return this.code instanceof r._CodeOrName ? this.code.names : {};
    }
  }
  class GQ extends X6 {
    constructor(Q = []) {
      super();
      this.nodes = Q;
    }
    render(Q) {
      return this.nodes.reduce((X, Y) => X + Y.render(Q), "");
    }
    optimizeNodes() {
      let { nodes: Q } = this, X = Q.length;
      while (X--) {
        let Y = Q[X].optimizeNodes();
        if (Array.isArray(Y)) Q.splice(X, 1, ...Y);
        else if (Y) Q[X] = Y;
        else Q.splice(X, 1);
      }
      return Q.length > 0 ? this : void 0;
    }
    optimizeNames(Q, X) {
      let { nodes: Y } = this, $ = Y.length;
      while ($--) {
        let J = Y[$];
        if (J.optimizeNames(Q, X)) continue;
        bM(Q, J.names), Y.splice($, 1);
      }
      return Y.length > 0 ? this : void 0;
    }
    get names() {
      return this.nodes.reduce((Q, X) => E6(Q, X.names), {});
    }
  }
  class Y6 extends GQ {
    render(Q) {
      return "{" + Q._n + super.render(Q) + "}" + Q._n;
    }
  }
  class M3 extends GQ {
  }
  class w4 extends Y6 {
  }
  w4.kind = "else";
  class _1 extends Y6 {
    constructor(Q, X) {
      super(X);
      this.condition = Q;
    }
    render(Q) {
      let X = `if(${this.condition})` + super.render(Q);
      if (this.else) X += "else " + this.else.render(Q);
      return X;
    }
    optimizeNodes() {
      super.optimizeNodes();
      let Q = this.condition;
      if (Q === true) return this.nodes;
      let X = this.else;
      if (X) {
        let Y = X.optimizeNodes();
        X = this.else = Array.isArray(Y) ? new w4(Y) : Y;
      }
      if (X) {
        if (Q === false) return X instanceof _1 ? X : X.nodes;
        if (this.nodes.length) return this;
        return new _1(I3(Q), X instanceof _1 ? [X] : X.nodes);
      }
      if (Q === false || !this.nodes.length) return;
      return this;
    }
    optimizeNames(Q, X) {
      var Y;
      if (this.else = (Y = this.else) === null || Y === void 0 ? void 0 : Y.optimizeNames(Q, X), !(super.optimizeNames(Q, X) || this.else)) return;
      return this.condition = A9(this.condition, Q, X), this;
    }
    get names() {
      let Q = super.names;
      if (JQ(Q, this.condition), this.else) E6(Q, this.else.names);
      return Q;
    }
  }
  _1.kind = "if";
  class w9 extends Y6 {
  }
  w9.kind = "for";
  class w3 extends w9 {
    constructor(Q) {
      super();
      this.iteration = Q;
    }
    render(Q) {
      return `for(${this.iteration})` + super.render(Q);
    }
    optimizeNames(Q, X) {
      if (!super.optimizeNames(Q, X)) return;
      return this.iteration = A9(this.iteration, Q, X), this;
    }
    get names() {
      return E6(super.names, this.iteration.names);
    }
  }
  class A3 extends w9 {
    constructor(Q, X, Y, $) {
      super();
      this.varKind = Q, this.name = X, this.from = Y, this.to = $;
    }
    render(Q) {
      let X = Q.es5 ? K1.varKinds.var : this.varKind, { name: Y, from: $, to: J } = this;
      return `for(${X} ${Y}=${$}; ${Y}<${J}; ${Y}++)` + super.render(Q);
    }
    get names() {
      let Q = JQ(super.names, this.from);
      return JQ(Q, this.to);
    }
  }
  class l$ extends w9 {
    constructor(Q, X, Y, $) {
      super();
      this.loop = Q, this.varKind = X, this.name = Y, this.iterable = $;
    }
    render(Q) {
      return `for(${this.varKind} ${this.name} ${this.loop} ${this.iterable})` + super.render(Q);
    }
    optimizeNames(Q, X) {
      if (!super.optimizeNames(Q, X)) return;
      return this.iterable = A9(this.iterable, Q, X), this;
    }
    get names() {
      return E6(super.names, this.iterable.names);
    }
  }
  class QQ extends Y6 {
    constructor(Q, X, Y) {
      super();
      this.name = Q, this.args = X, this.async = Y;
    }
    render(Q) {
      return `${this.async ? "async " : ""}function ${this.name}(${this.args})` + super.render(Q);
    }
  }
  QQ.kind = "func";
  class XQ extends GQ {
    render(Q) {
      return "return " + super.render(Q);
    }
  }
  XQ.kind = "return";
  class j3 extends Y6 {
    render(Q) {
      let X = "try" + super.render(Q);
      if (this.catch) X += this.catch.render(Q);
      if (this.finally) X += this.finally.render(Q);
      return X;
    }
    optimizeNodes() {
      var Q, X;
      return super.optimizeNodes(), (Q = this.catch) === null || Q === void 0 || Q.optimizeNodes(), (X = this.finally) === null || X === void 0 || X.optimizeNodes(), this;
    }
    optimizeNames(Q, X) {
      var Y, $;
      return super.optimizeNames(Q, X), (Y = this.catch) === null || Y === void 0 || Y.optimizeNames(Q, X), ($ = this.finally) === null || $ === void 0 || $.optimizeNames(Q, X), this;
    }
    get names() {
      let Q = super.names;
      if (this.catch) E6(Q, this.catch.names);
      if (this.finally) E6(Q, this.finally.names);
      return Q;
    }
  }
  class YQ extends Y6 {
    constructor(Q) {
      super();
      this.error = Q;
    }
    render(Q) {
      return `catch(${this.error})` + super.render(Q);
    }
  }
  YQ.kind = "catch";
  class $Q extends Y6 {
    render(Q) {
      return "finally" + super.render(Q);
    }
  }
  $Q.kind = "finally";
  class R3 {
    constructor(Q, X = {}) {
      this._values = {}, this._blockStarts = [], this._constants = {}, this.opts = { ...X, _n: X.lines ? `
` : "" }, this._extScope = Q, this._scope = new K1.Scope({ parent: Q }), this._nodes = [new M3()];
    }
    toString() {
      return this._root.render(this.opts);
    }
    name(Q) {
      return this._scope.name(Q);
    }
    scopeName(Q) {
      return this._extScope.name(Q);
    }
    scopeValue(Q, X) {
      let Y = this._extScope.value(Q, X);
      return (this._values[Y.prefix] || (this._values[Y.prefix] = /* @__PURE__ */ new Set())).add(Y), Y;
    }
    getScopeValue(Q, X) {
      return this._extScope.getValue(Q, X);
    }
    scopeRefs(Q) {
      return this._extScope.scopeRefs(Q, this._values);
    }
    scopeCode() {
      return this._extScope.scopeCode(this._values);
    }
    _def(Q, X, Y, $) {
      let J = this._scope.toName(X);
      if (Y !== void 0 && $) this._constants[J.str] = Y;
      return this._leafNode(new U3(Q, J, Y)), J;
    }
    const(Q, X, Y) {
      return this._def(K1.varKinds.const, Q, X, Y);
    }
    let(Q, X, Y) {
      return this._def(K1.varKinds.let, Q, X, Y);
    }
    var(Q, X, Y) {
      return this._def(K1.varKinds.var, Q, X, Y);
    }
    assign(Q, X, Y) {
      return this._leafNode(new p$(Q, X, Y));
    }
    add(Q, X) {
      return this._leafNode(new L3(Q, h0.operators.ADD, X));
    }
    code(Q) {
      if (typeof Q == "function") Q();
      else if (Q !== r.nil) this._leafNode(new D3(Q));
      return this;
    }
    object(...Q) {
      let X = ["{"];
      for (let [Y, $] of Q) {
        if (X.length > 1) X.push(",");
        if (X.push(Y), Y !== $ || this.opts.es5) X.push(":"), (0, r.addCodeArg)(X, $);
      }
      return X.push("}"), new r._Code(X);
    }
    if(Q, X, Y) {
      if (this._blockNode(new _1(Q)), X && Y) this.code(X).else().code(Y).endIf();
      else if (X) this.code(X).endIf();
      else if (Y) throw Error('CodeGen: "else" body without "then" body');
      return this;
    }
    elseIf(Q) {
      return this._elseNode(new _1(Q));
    }
    else() {
      return this._elseNode(new w4());
    }
    endIf() {
      return this._endBlockNode(_1, w4);
    }
    _for(Q, X) {
      if (this._blockNode(Q), X) this.code(X).endFor();
      return this;
    }
    for(Q, X) {
      return this._for(new w3(Q), X);
    }
    forRange(Q, X, Y, $, J = this.opts.es5 ? K1.varKinds.var : K1.varKinds.let) {
      let W = this._scope.toName(Q);
      return this._for(new A3(J, W, X, Y), () => $(W));
    }
    forOf(Q, X, Y, $ = K1.varKinds.const) {
      let J = this._scope.toName(Q);
      if (this.opts.es5) {
        let W = X instanceof r.Name ? X : this.var("_arr", X);
        return this.forRange("_i", 0, r._`${W}.length`, (G) => {
          this.var(J, r._`${W}[${G}]`), Y(J);
        });
      }
      return this._for(new l$("of", $, J, X), () => Y(J));
    }
    forIn(Q, X, Y, $ = this.opts.es5 ? K1.varKinds.var : K1.varKinds.const) {
      if (this.opts.ownProperties) return this.forOf(Q, r._`Object.keys(${X})`, Y);
      let J = this._scope.toName(Q);
      return this._for(new l$("in", $, J, X), () => Y(J));
    }
    endFor() {
      return this._endBlockNode(w9);
    }
    label(Q) {
      return this._leafNode(new F3(Q));
    }
    break(Q) {
      return this._leafNode(new N3(Q));
    }
    return(Q) {
      let X = new XQ();
      if (this._blockNode(X), this.code(Q), X.nodes.length !== 1) throw Error('CodeGen: "return" should have one node');
      return this._endBlockNode(XQ);
    }
    try(Q, X, Y) {
      if (!X && !Y) throw Error('CodeGen: "try" without "catch" and "finally"');
      let $ = new j3();
      if (this._blockNode($), this.code(Q), X) {
        let J = this.name("e");
        this._currNode = $.catch = new YQ(J), X(J);
      }
      if (Y) this._currNode = $.finally = new $Q(), this.code(Y);
      return this._endBlockNode(YQ, $Q);
    }
    throw(Q) {
      return this._leafNode(new O3(Q));
    }
    block(Q, X) {
      if (this._blockStarts.push(this._nodes.length), Q) this.code(Q).endBlock(X);
      return this;
    }
    endBlock(Q) {
      let X = this._blockStarts.pop();
      if (X === void 0) throw Error("CodeGen: not in self-balancing block");
      let Y = this._nodes.length - X;
      if (Y < 0 || Q !== void 0 && Y !== Q) throw Error(`CodeGen: wrong number of nodes: ${Y} vs ${Q} expected`);
      return this._nodes.length = X, this;
    }
    func(Q, X = r.nil, Y, $) {
      if (this._blockNode(new QQ(Q, X, Y)), $) this.code($).endFunc();
      return this;
    }
    endFunc() {
      return this._endBlockNode(QQ);
    }
    optimize(Q = 1) {
      while (Q-- > 0) this._root.optimizeNodes(), this._root.optimizeNames(this._root.names, this._constants);
    }
    _leafNode(Q) {
      return this._currNode.nodes.push(Q), this;
    }
    _blockNode(Q) {
      this._currNode.nodes.push(Q), this._nodes.push(Q);
    }
    _endBlockNode(Q, X) {
      let Y = this._currNode;
      if (Y instanceof Q || X && Y instanceof X) return this._nodes.pop(), this;
      throw Error(`CodeGen: not in block "${X ? `${Q.kind}/${X.kind}` : Q.kind}"`);
    }
    _elseNode(Q) {
      let X = this._currNode;
      if (!(X instanceof _1)) throw Error('CodeGen: "else" without "if"');
      return this._currNode = X.else = Q, this;
    }
    get _root() {
      return this._nodes[0];
    }
    get _currNode() {
      let Q = this._nodes;
      return Q[Q.length - 1];
    }
    set _currNode(Q) {
      let X = this._nodes;
      X[X.length - 1] = Q;
    }
  }
  h0.CodeGen = R3;
  function E6(Q, X) {
    for (let Y in X) Q[Y] = (Q[Y] || 0) + (X[Y] || 0);
    return Q;
  }
  function JQ(Q, X) {
    return X instanceof r._CodeOrName ? E6(Q, X.names) : Q;
  }
  function A9(Q, X, Y) {
    if (Q instanceof r.Name) return $(Q);
    if (!J(Q)) return Q;
    return new r._Code(Q._items.reduce((W, G) => {
      if (G instanceof r.Name) G = $(G);
      if (G instanceof r._Code) W.push(...G._items);
      else W.push(G);
      return W;
    }, []));
    function $(W) {
      let G = Y[W.str];
      if (G === void 0 || X[W.str] !== 1) return W;
      return delete X[W.str], G;
    }
    function J(W) {
      return W instanceof r._Code && W._items.some((G) => G instanceof r.Name && X[G.str] === 1 && Y[G.str] !== void 0);
    }
  }
  function bM(Q, X) {
    for (let Y in X) Q[Y] = (Q[Y] || 0) - (X[Y] || 0);
  }
  function I3(Q) {
    return typeof Q == "boolean" || typeof Q == "number" || Q === null ? !Q : r._`!${c$(Q)}`;
  }
  h0.not = I3;
  var EM = b3(h0.operators.AND);
  function PM(...Q) {
    return Q.reduce(EM);
  }
  h0.and = PM;
  var ZM = b3(h0.operators.OR);
  function CM(...Q) {
    return Q.reduce(ZM);
  }
  h0.or = CM;
  function b3(Q) {
    return (X, Y) => X === r.nil ? Y : Y === r.nil ? X : r._`${c$(X)} ${Q} ${c$(Y)}`;
  }
  function c$(Q) {
    return Q instanceof r.Name ? Q : r._`(${Q})`;
  }
});
var a = E((T3) => {
  Object.defineProperty(T3, "__esModule", { value: true });
  T3.checkStrictMode = T3.getErrorPath = T3.Type = T3.useFunc = T3.setEvaluated = T3.evaluatedPropsToName = T3.mergeEvaluated = T3.eachItem = T3.unescapeJsonPointer = T3.escapeJsonPointer = T3.escapeFragment = T3.unescapeFragment = T3.schemaRefOrVal = T3.schemaHasRulesButRef = T3.schemaHasRules = T3.checkUnknownRules = T3.alwaysValidSchema = T3.toHash = void 0;
  var Q0 = c(), vM = M4();
  function TM(Q) {
    let X = {};
    for (let Y of Q) X[Y] = true;
    return X;
  }
  T3.toHash = TM;
  function xM(Q, X) {
    if (typeof X == "boolean") return X;
    if (Object.keys(X).length === 0) return true;
    return C3(Q, X), !S3(X, Q.self.RULES.all);
  }
  T3.alwaysValidSchema = xM;
  function C3(Q, X = Q.schema) {
    let { opts: Y, self: $ } = Q;
    if (!Y.strictSchema) return;
    if (typeof X === "boolean") return;
    let J = $.RULES.keywords;
    for (let W in X) if (!J[W]) v3(Q, `unknown keyword: "${W}"`);
  }
  T3.checkUnknownRules = C3;
  function S3(Q, X) {
    if (typeof Q == "boolean") return !Q;
    for (let Y in Q) if (X[Y]) return true;
    return false;
  }
  T3.schemaHasRules = S3;
  function yM(Q, X) {
    if (typeof Q == "boolean") return !Q;
    for (let Y in Q) if (Y !== "$ref" && X.all[Y]) return true;
    return false;
  }
  T3.schemaHasRulesButRef = yM;
  function gM({ topSchemaRef: Q, schemaPath: X }, Y, $, J) {
    if (!J) {
      if (typeof Y == "number" || typeof Y == "boolean") return Y;
      if (typeof Y == "string") return Q0._`${Y}`;
    }
    return Q0._`${Q}${X}${(0, Q0.getProperty)($)}`;
  }
  T3.schemaRefOrVal = gM;
  function hM(Q) {
    return _3(decodeURIComponent(Q));
  }
  T3.unescapeFragment = hM;
  function fM(Q) {
    return encodeURIComponent(i$(Q));
  }
  T3.escapeFragment = fM;
  function i$(Q) {
    if (typeof Q == "number") return `${Q}`;
    return Q.replace(/~/g, "~0").replace(/\//g, "~1");
  }
  T3.escapeJsonPointer = i$;
  function _3(Q) {
    return Q.replace(/~1/g, "/").replace(/~0/g, "~");
  }
  T3.unescapeJsonPointer = _3;
  function uM(Q, X) {
    if (Array.isArray(Q)) for (let Y of Q) X(Y);
    else X(Q);
  }
  T3.eachItem = uM;
  function P3({ mergeNames: Q, mergeToName: X, mergeValues: Y, resultToName: $ }) {
    return (J, W, G, H) => {
      let B = G === void 0 ? W : G instanceof Q0.Name ? (W instanceof Q0.Name ? Q(J, W, G) : X(J, W, G), G) : W instanceof Q0.Name ? (X(J, G, W), W) : Y(W, G);
      return H === Q0.Name && !(B instanceof Q0.Name) ? $(J, B) : B;
    };
  }
  T3.mergeEvaluated = { props: P3({ mergeNames: (Q, X, Y) => Q.if(Q0._`${Y} !== true && ${X} !== undefined`, () => {
    Q.if(Q0._`${X} === true`, () => Q.assign(Y, true), () => Q.assign(Y, Q0._`${Y} || {}`).code(Q0._`Object.assign(${Y}, ${X})`));
  }), mergeToName: (Q, X, Y) => Q.if(Q0._`${Y} !== true`, () => {
    if (X === true) Q.assign(Y, true);
    else Q.assign(Y, Q0._`${Y} || {}`), n$(Q, Y, X);
  }), mergeValues: (Q, X) => Q === true ? true : { ...Q, ...X }, resultToName: k3 }), items: P3({ mergeNames: (Q, X, Y) => Q.if(Q0._`${Y} !== true && ${X} !== undefined`, () => Q.assign(Y, Q0._`${X} === true ? true : ${Y} > ${X} ? ${Y} : ${X}`)), mergeToName: (Q, X, Y) => Q.if(Q0._`${Y} !== true`, () => Q.assign(Y, X === true ? true : Q0._`${Y} > ${X} ? ${Y} : ${X}`)), mergeValues: (Q, X) => Q === true ? true : Math.max(Q, X), resultToName: (Q, X) => Q.var("items", X) }) };
  function k3(Q, X) {
    if (X === true) return Q.var("props", true);
    let Y = Q.var("props", Q0._`{}`);
    if (X !== void 0) n$(Q, Y, X);
    return Y;
  }
  T3.evaluatedPropsToName = k3;
  function n$(Q, X, Y) {
    Object.keys(Y).forEach(($) => Q.assign(Q0._`${X}${(0, Q0.getProperty)($)}`, true));
  }
  T3.setEvaluated = n$;
  var Z3 = {};
  function mM(Q, X) {
    return Q.scopeValue("func", { ref: X, code: Z3[X.code] || (Z3[X.code] = new vM._Code(X.code)) });
  }
  T3.useFunc = mM;
  var d$;
  (function(Q) {
    Q[Q.Num = 0] = "Num", Q[Q.Str = 1] = "Str";
  })(d$ || (T3.Type = d$ = {}));
  function lM(Q, X, Y) {
    if (Q instanceof Q0.Name) {
      let $ = X === d$.Num;
      return Y ? $ ? Q0._`"[" + ${Q} + "]"` : Q0._`"['" + ${Q} + "']"` : $ ? Q0._`"/" + ${Q}` : Q0._`"/" + ${Q}.replace(/~/g, "~0").replace(/\\//g, "~1")`;
    }
    return Y ? (0, Q0.getProperty)(Q).toString() : "/" + i$(Q);
  }
  T3.getErrorPath = lM;
  function v3(Q, X, Y = Q.opts.strictSchema) {
    if (!Y) return;
    if (X = `strict mode: ${X}`, Y === true) throw Error(X);
    Q.self.logger.warn(X);
  }
  T3.checkStrictMode = v3;
});
var k1 = E((y3) => {
  Object.defineProperty(y3, "__esModule", { value: true });
  var Z0 = c(), Gw = { data: new Z0.Name("data"), valCxt: new Z0.Name("valCxt"), instancePath: new Z0.Name("instancePath"), parentData: new Z0.Name("parentData"), parentDataProperty: new Z0.Name("parentDataProperty"), rootData: new Z0.Name("rootData"), dynamicAnchors: new Z0.Name("dynamicAnchors"), vErrors: new Z0.Name("vErrors"), errors: new Z0.Name("errors"), this: new Z0.Name("this"), self: new Z0.Name("self"), scope: new Z0.Name("scope"), json: new Z0.Name("json"), jsonPos: new Z0.Name("jsonPos"), jsonLen: new Z0.Name("jsonLen"), jsonPart: new Z0.Name("jsonPart") };
  y3.default = Gw;
});
var A4 = E((u3) => {
  Object.defineProperty(u3, "__esModule", { value: true });
  u3.extendErrors = u3.resetErrorsCount = u3.reportExtraError = u3.reportError = u3.keyword$DataError = u3.keywordError = void 0;
  var t = c(), BQ = a(), k0 = k1();
  u3.keywordError = { message: ({ keyword: Q }) => t.str`must pass "${Q}" keyword validation` };
  u3.keyword$DataError = { message: ({ keyword: Q, schemaType: X }) => X ? t.str`"${Q}" keyword must be ${X} ($data)` : t.str`"${Q}" keyword is invalid ($data)` };
  function Bw(Q, X = u3.keywordError, Y, $) {
    let { it: J } = Q, { gen: W, compositeRule: G, allErrors: H } = J, B = f3(Q, X, Y);
    if ($ !== null && $ !== void 0 ? $ : G || H) g3(W, B);
    else h3(J, t._`[${B}]`);
  }
  u3.reportError = Bw;
  function zw(Q, X = u3.keywordError, Y) {
    let { it: $ } = Q, { gen: J, compositeRule: W, allErrors: G } = $, H = f3(Q, X, Y);
    if (g3(J, H), !(W || G)) h3($, k0.default.vErrors);
  }
  u3.reportExtraError = zw;
  function Kw(Q, X) {
    Q.assign(k0.default.errors, X), Q.if(t._`${k0.default.vErrors} !== null`, () => Q.if(X, () => Q.assign(t._`${k0.default.vErrors}.length`, X), () => Q.assign(k0.default.vErrors, null)));
  }
  u3.resetErrorsCount = Kw;
  function Vw({ gen: Q, keyword: X, schemaValue: Y, data: $, errsCount: J, it: W }) {
    if (J === void 0) throw Error("ajv implementation error");
    let G = Q.name("err");
    Q.forRange("i", J, k0.default.errors, (H) => {
      if (Q.const(G, t._`${k0.default.vErrors}[${H}]`), Q.if(t._`${G}.instancePath === undefined`, () => Q.assign(t._`${G}.instancePath`, (0, t.strConcat)(k0.default.instancePath, W.errorPath))), Q.assign(t._`${G}.schemaPath`, t.str`${W.errSchemaPath}/${X}`), W.opts.verbose) Q.assign(t._`${G}.schema`, Y), Q.assign(t._`${G}.data`, $);
    });
  }
  u3.extendErrors = Vw;
  function g3(Q, X) {
    let Y = Q.const("err", X);
    Q.if(t._`${k0.default.vErrors} === null`, () => Q.assign(k0.default.vErrors, t._`[${Y}]`), t._`${k0.default.vErrors}.push(${Y})`), Q.code(t._`${k0.default.errors}++`);
  }
  function h3(Q, X) {
    let { gen: Y, validateName: $, schemaEnv: J } = Q;
    if (J.$async) Y.throw(t._`new ${Q.ValidationError}(${X})`);
    else Y.assign(t._`${$}.errors`, X), Y.return(false);
  }
  var P6 = { keyword: new t.Name("keyword"), schemaPath: new t.Name("schemaPath"), params: new t.Name("params"), propertyName: new t.Name("propertyName"), message: new t.Name("message"), schema: new t.Name("schema"), parentSchema: new t.Name("parentSchema") };
  function f3(Q, X, Y) {
    let { createErrors: $ } = Q.it;
    if ($ === false) return t._`{}`;
    return qw(Q, X, Y);
  }
  function qw(Q, X, Y = {}) {
    let { gen: $, it: J } = Q, W = [Uw(J, Y), Lw(Q, Y)];
    return Fw(Q, X, W), $.object(...W);
  }
  function Uw({ errorPath: Q }, { instancePath: X }) {
    let Y = X ? t.str`${Q}${(0, BQ.getErrorPath)(X, BQ.Type.Str)}` : Q;
    return [k0.default.instancePath, (0, t.strConcat)(k0.default.instancePath, Y)];
  }
  function Lw({ keyword: Q, it: { errSchemaPath: X } }, { schemaPath: Y, parentSchema: $ }) {
    let J = $ ? X : t.str`${X}/${Q}`;
    if (Y) J = t.str`${J}${(0, BQ.getErrorPath)(Y, BQ.Type.Str)}`;
    return [P6.schemaPath, J];
  }
  function Fw(Q, { params: X, message: Y }, $) {
    let { keyword: J, data: W, schemaValue: G, it: H } = Q, { opts: B, propertyName: z, topSchemaRef: K, schemaPath: U } = H;
    if ($.push([P6.keyword, J], [P6.params, typeof X == "function" ? X(Q) : X || t._`{}`]), B.messages) $.push([P6.message, typeof Y == "function" ? Y(Q) : Y]);
    if (B.verbose) $.push([P6.schema, G], [P6.parentSchema, t._`${K}${U}`], [k0.default.data, W]);
    if (z) $.push([P6.propertyName, z]);
  }
});
var d3 = E((c3) => {
  Object.defineProperty(c3, "__esModule", { value: true });
  c3.boolOrEmptySchema = c3.topBoolOrEmptySchema = void 0;
  var ww = A4(), Aw = c(), jw = k1(), Rw = { message: "boolean schema is false" };
  function Iw(Q) {
    let { gen: X, schema: Y, validateName: $ } = Q;
    if (Y === false) l3(Q, false);
    else if (typeof Y == "object" && Y.$async === true) X.return(jw.default.data);
    else X.assign(Aw._`${$}.errors`, null), X.return(true);
  }
  c3.topBoolOrEmptySchema = Iw;
  function bw(Q, X) {
    let { gen: Y, schema: $ } = Q;
    if ($ === false) Y.var(X, false), l3(Q);
    else Y.var(X, true);
  }
  c3.boolOrEmptySchema = bw;
  function l3(Q, X) {
    let { gen: Y, data: $ } = Q, J = { gen: Y, keyword: "false schema", data: $, schema: false, schemaCode: false, schemaValue: false, params: {}, it: Q };
    (0, ww.reportError)(J, Rw, void 0, X);
  }
});
var r$ = E((i3) => {
  Object.defineProperty(i3, "__esModule", { value: true });
  i3.getRules = i3.isJSONType = void 0;
  var Pw = ["string", "number", "integer", "boolean", "null", "object", "array"], Zw = new Set(Pw);
  function Cw(Q) {
    return typeof Q == "string" && Zw.has(Q);
  }
  i3.isJSONType = Cw;
  function Sw() {
    let Q = { number: { type: "number", rules: [] }, string: { type: "string", rules: [] }, array: { type: "array", rules: [] }, object: { type: "object", rules: [] } };
    return { types: { ...Q, integer: true, boolean: true, null: true }, rules: [{ rules: [] }, Q.number, Q.string, Q.array, Q.object], post: { rules: [] }, all: {}, keywords: {} };
  }
  i3.getRules = Sw;
});
var t$ = E((t3) => {
  Object.defineProperty(t3, "__esModule", { value: true });
  t3.shouldUseRule = t3.shouldUseGroup = t3.schemaHasRulesForType = void 0;
  function kw({ schema: Q, self: X }, Y) {
    let $ = X.RULES.types[Y];
    return $ && $ !== true && o3(Q, $);
  }
  t3.schemaHasRulesForType = kw;
  function o3(Q, X) {
    return X.rules.some((Y) => r3(Q, Y));
  }
  t3.shouldUseGroup = o3;
  function r3(Q, X) {
    var Y;
    return Q[X.keyword] !== void 0 || ((Y = X.definition.implements) === null || Y === void 0 ? void 0 : Y.some(($) => Q[$] !== void 0));
  }
  t3.shouldUseRule = r3;
});
var j4 = E((XH) => {
  Object.defineProperty(XH, "__esModule", { value: true });
  XH.reportTypeError = XH.checkDataTypes = XH.checkDataType = XH.coerceAndCheckDataType = XH.getJSONTypes = XH.getSchemaTypes = XH.DataType = void 0;
  var xw = r$(), yw = t$(), gw = A4(), l = c(), s3 = a(), j9;
  (function(Q) {
    Q[Q.Correct = 0] = "Correct", Q[Q.Wrong = 1] = "Wrong";
  })(j9 || (XH.DataType = j9 = {}));
  function hw(Q) {
    let X = e3(Q.type);
    if (X.includes("null")) {
      if (Q.nullable === false) throw Error("type: null contradicts nullable: false");
    } else {
      if (!X.length && Q.nullable !== void 0) throw Error('"nullable" cannot be used without "type"');
      if (Q.nullable === true) X.push("null");
    }
    return X;
  }
  XH.getSchemaTypes = hw;
  function e3(Q) {
    let X = Array.isArray(Q) ? Q : Q ? [Q] : [];
    if (X.every(xw.isJSONType)) return X;
    throw Error("type must be JSONType or JSONType[]: " + X.join(","));
  }
  XH.getJSONTypes = e3;
  function fw(Q, X) {
    let { gen: Y, data: $, opts: J } = Q, W = uw(X, J.coerceTypes), G = X.length > 0 && !(W.length === 0 && X.length === 1 && (0, yw.schemaHasRulesForType)(Q, X[0]));
    if (G) {
      let H = s$(X, $, J.strictNumbers, j9.Wrong);
      Y.if(H, () => {
        if (W.length) mw(Q, X, W);
        else e$(Q);
      });
    }
    return G;
  }
  XH.coerceAndCheckDataType = fw;
  var QH = /* @__PURE__ */ new Set(["string", "number", "integer", "boolean", "null"]);
  function uw(Q, X) {
    return X ? Q.filter((Y) => QH.has(Y) || X === "array" && Y === "array") : [];
  }
  function mw(Q, X, Y) {
    let { gen: $, data: J, opts: W } = Q, G = $.let("dataType", l._`typeof ${J}`), H = $.let("coerced", l._`undefined`);
    if (W.coerceTypes === "array") $.if(l._`${G} == 'object' && Array.isArray(${J}) && ${J}.length == 1`, () => $.assign(J, l._`${J}[0]`).assign(G, l._`typeof ${J}`).if(s$(X, J, W.strictNumbers), () => $.assign(H, J)));
    $.if(l._`${H} !== undefined`);
    for (let z of Y) if (QH.has(z) || z === "array" && W.coerceTypes === "array") B(z);
    $.else(), e$(Q), $.endIf(), $.if(l._`${H} !== undefined`, () => {
      $.assign(J, H), lw(Q, H);
    });
    function B(z) {
      switch (z) {
        case "string":
          $.elseIf(l._`${G} == "number" || ${G} == "boolean"`).assign(H, l._`"" + ${J}`).elseIf(l._`${J} === null`).assign(H, l._`""`);
          return;
        case "number":
          $.elseIf(l._`${G} == "boolean" || ${J} === null
              || (${G} == "string" && ${J} && ${J} == +${J})`).assign(H, l._`+${J}`);
          return;
        case "integer":
          $.elseIf(l._`${G} === "boolean" || ${J} === null
              || (${G} === "string" && ${J} && ${J} == +${J} && !(${J} % 1))`).assign(H, l._`+${J}`);
          return;
        case "boolean":
          $.elseIf(l._`${J} === "false" || ${J} === 0 || ${J} === null`).assign(H, false).elseIf(l._`${J} === "true" || ${J} === 1`).assign(H, true);
          return;
        case "null":
          $.elseIf(l._`${J} === "" || ${J} === 0 || ${J} === false`), $.assign(H, null);
          return;
        case "array":
          $.elseIf(l._`${G} === "string" || ${G} === "number"
              || ${G} === "boolean" || ${J} === null`).assign(H, l._`[${J}]`);
      }
    }
  }
  function lw({ gen: Q, parentData: X, parentDataProperty: Y }, $) {
    Q.if(l._`${X} !== undefined`, () => Q.assign(l._`${X}[${Y}]`, $));
  }
  function a$(Q, X, Y, $ = j9.Correct) {
    let J = $ === j9.Correct ? l.operators.EQ : l.operators.NEQ, W;
    switch (Q) {
      case "null":
        return l._`${X} ${J} null`;
      case "array":
        W = l._`Array.isArray(${X})`;
        break;
      case "object":
        W = l._`${X} && typeof ${X} == "object" && !Array.isArray(${X})`;
        break;
      case "integer":
        W = G(l._`!(${X} % 1) && !isNaN(${X})`);
        break;
      case "number":
        W = G();
        break;
      default:
        return l._`typeof ${X} ${J} ${Q}`;
    }
    return $ === j9.Correct ? W : (0, l.not)(W);
    function G(H = l.nil) {
      return (0, l.and)(l._`typeof ${X} == "number"`, H, Y ? l._`isFinite(${X})` : l.nil);
    }
  }
  XH.checkDataType = a$;
  function s$(Q, X, Y, $) {
    if (Q.length === 1) return a$(Q[0], X, Y, $);
    let J, W = (0, s3.toHash)(Q);
    if (W.array && W.object) {
      let G = l._`typeof ${X} != "object"`;
      J = W.null ? G : l._`!${X} || ${G}`, delete W.null, delete W.array, delete W.object;
    } else J = l.nil;
    if (W.number) delete W.integer;
    for (let G in W) J = (0, l.and)(J, a$(G, X, Y, $));
    return J;
  }
  XH.checkDataTypes = s$;
  var cw = { message: ({ schema: Q }) => `must be ${Q}`, params: ({ schema: Q, schemaValue: X }) => typeof Q == "string" ? l._`{type: ${Q}}` : l._`{type: ${X}}` };
  function e$(Q) {
    let X = pw(Q);
    (0, gw.reportError)(X, cw);
  }
  XH.reportTypeError = e$;
  function pw(Q) {
    let { gen: X, data: Y, schema: $ } = Q, J = (0, s3.schemaRefOrVal)(Q, $, "type");
    return { gen: X, keyword: "type", data: Y, schema: $.type, schemaCode: J, schemaValue: J, parentSchema: $, params: {}, it: Q };
  }
});
var GH = E((JH) => {
  Object.defineProperty(JH, "__esModule", { value: true });
  JH.assignDefaults = void 0;
  var R9 = c(), aw = a();
  function sw(Q, X) {
    let { properties: Y, items: $ } = Q.schema;
    if (X === "object" && Y) for (let J in Y) $H(Q, J, Y[J].default);
    else if (X === "array" && Array.isArray($)) $.forEach((J, W) => $H(Q, W, J.default));
  }
  JH.assignDefaults = sw;
  function $H(Q, X, Y) {
    let { gen: $, compositeRule: J, data: W, opts: G } = Q;
    if (Y === void 0) return;
    let H = R9._`${W}${(0, R9.getProperty)(X)}`;
    if (J) {
      (0, aw.checkStrictMode)(Q, `default is ignored for: ${H}`);
      return;
    }
    let B = R9._`${H} === undefined`;
    if (G.useDefaults === "empty") B = R9._`${B} || ${H} === null || ${H} === ""`;
    $.if(B, R9._`${H} = ${(0, R9.stringify)(Y)}`);
  }
});
var e0 = E((zH) => {
  Object.defineProperty(zH, "__esModule", { value: true });
  zH.validateUnion = zH.validateArray = zH.usePattern = zH.callValidateCode = zH.schemaProperties = zH.allSchemaProperties = zH.noPropertyInData = zH.propertyInData = zH.isOwnProperty = zH.hasPropFunc = zH.reportMissingProp = zH.checkMissingProp = zH.checkReportMissingProp = void 0;
  var H0 = c(), Q7 = a(), $6 = k1(), ew = a();
  function QA(Q, X) {
    let { gen: Y, data: $, it: J } = Q;
    Y.if(Y7(Y, $, X, J.opts.ownProperties), () => {
      Q.setParams({ missingProperty: H0._`${X}` }, true), Q.error();
    });
  }
  zH.checkReportMissingProp = QA;
  function XA({ gen: Q, data: X, it: { opts: Y } }, $, J) {
    return (0, H0.or)(...$.map((W) => (0, H0.and)(Y7(Q, X, W, Y.ownProperties), H0._`${J} = ${W}`)));
  }
  zH.checkMissingProp = XA;
  function YA(Q, X) {
    Q.setParams({ missingProperty: X }, true), Q.error();
  }
  zH.reportMissingProp = YA;
  function HH(Q) {
    return Q.scopeValue("func", { ref: Object.prototype.hasOwnProperty, code: H0._`Object.prototype.hasOwnProperty` });
  }
  zH.hasPropFunc = HH;
  function X7(Q, X, Y) {
    return H0._`${HH(Q)}.call(${X}, ${Y})`;
  }
  zH.isOwnProperty = X7;
  function $A(Q, X, Y, $) {
    let J = H0._`${X}${(0, H0.getProperty)(Y)} !== undefined`;
    return $ ? H0._`${J} && ${X7(Q, X, Y)}` : J;
  }
  zH.propertyInData = $A;
  function Y7(Q, X, Y, $) {
    let J = H0._`${X}${(0, H0.getProperty)(Y)} === undefined`;
    return $ ? (0, H0.or)(J, (0, H0.not)(X7(Q, X, Y))) : J;
  }
  zH.noPropertyInData = Y7;
  function BH(Q) {
    return Q ? Object.keys(Q).filter((X) => X !== "__proto__") : [];
  }
  zH.allSchemaProperties = BH;
  function JA(Q, X) {
    return BH(X).filter((Y) => !(0, Q7.alwaysValidSchema)(Q, X[Y]));
  }
  zH.schemaProperties = JA;
  function WA({ schemaCode: Q, data: X, it: { gen: Y, topSchemaRef: $, schemaPath: J, errorPath: W }, it: G }, H, B, z) {
    let K = z ? H0._`${Q}, ${X}, ${$}${J}` : X, U = [[$6.default.instancePath, (0, H0.strConcat)($6.default.instancePath, W)], [$6.default.parentData, G.parentData], [$6.default.parentDataProperty, G.parentDataProperty], [$6.default.rootData, $6.default.rootData]];
    if (G.opts.dynamicRef) U.push([$6.default.dynamicAnchors, $6.default.dynamicAnchors]);
    let q = H0._`${K}, ${Y.object(...U)}`;
    return B !== H0.nil ? H0._`${H}.call(${B}, ${q})` : H0._`${H}(${q})`;
  }
  zH.callValidateCode = WA;
  var GA = H0._`new RegExp`;
  function HA({ gen: Q, it: { opts: X } }, Y) {
    let $ = X.unicodeRegExp ? "u" : "", { regExp: J } = X.code, W = J(Y, $);
    return Q.scopeValue("pattern", { key: W.toString(), ref: W, code: H0._`${J.code === "new RegExp" ? GA : (0, ew.useFunc)(Q, J)}(${Y}, ${$})` });
  }
  zH.usePattern = HA;
  function BA(Q) {
    let { gen: X, data: Y, keyword: $, it: J } = Q, W = X.name("valid");
    if (J.allErrors) {
      let H = X.let("valid", true);
      return G(() => X.assign(H, false)), H;
    }
    return X.var(W, true), G(() => X.break()), W;
    function G(H) {
      let B = X.const("len", H0._`${Y}.length`);
      X.forRange("i", 0, B, (z) => {
        Q.subschema({ keyword: $, dataProp: z, dataPropType: Q7.Type.Num }, W), X.if((0, H0.not)(W), H);
      });
    }
  }
  zH.validateArray = BA;
  function zA(Q) {
    let { gen: X, schema: Y, keyword: $, it: J } = Q;
    if (!Array.isArray(Y)) throw Error("ajv implementation error");
    if (Y.some((B) => (0, Q7.alwaysValidSchema)(J, B)) && !J.opts.unevaluated) return;
    let G = X.let("valid", false), H = X.name("_valid");
    X.block(() => Y.forEach((B, z) => {
      let K = Q.subschema({ keyword: $, schemaProp: z, compositeRule: true }, H);
      if (X.assign(G, H0._`${G} || ${H}`), !Q.mergeValidEvaluated(K, H)) X.if((0, H0.not)(G));
    })), Q.result(G, () => Q.reset(), () => Q.error(true));
  }
  zH.validateUnion = zA;
});
var FH = E((UH) => {
  Object.defineProperty(UH, "__esModule", { value: true });
  UH.validateKeywordUsage = UH.validSchemaType = UH.funcKeywordCode = UH.macroKeywordCode = void 0;
  var v0 = c(), Z6 = k1(), jA = e0(), RA = A4();
  function IA(Q, X) {
    let { gen: Y, keyword: $, schema: J, parentSchema: W, it: G } = Q, H = X.macro.call(G.self, J, W, G), B = qH(Y, $, H);
    if (G.opts.validateSchema !== false) G.self.validateSchema(H, true);
    let z = Y.name("valid");
    Q.subschema({ schema: H, schemaPath: v0.nil, errSchemaPath: `${G.errSchemaPath}/${$}`, topSchemaRef: B, compositeRule: true }, z), Q.pass(z, () => Q.error(true));
  }
  UH.macroKeywordCode = IA;
  function bA(Q, X) {
    var Y;
    let { gen: $, keyword: J, schema: W, parentSchema: G, $data: H, it: B } = Q;
    PA(B, X);
    let z = !H && X.compile ? X.compile.call(B.self, W, G, B) : X.validate, K = qH($, J, z), U = $.let("valid");
    Q.block$data(U, q), Q.ok((Y = X.valid) !== null && Y !== void 0 ? Y : U);
    function q() {
      if (X.errors === false) {
        if (F(), X.modifying) VH(Q);
        M(() => Q.error());
      } else {
        let O = X.async ? V() : L();
        if (X.modifying) VH(Q);
        M(() => EA(Q, O));
      }
    }
    function V() {
      let O = $.let("ruleErrs", null);
      return $.try(() => F(v0._`await `), (A) => $.assign(U, false).if(v0._`${A} instanceof ${B.ValidationError}`, () => $.assign(O, v0._`${A}.errors`), () => $.throw(A))), O;
    }
    function L() {
      let O = v0._`${K}.errors`;
      return $.assign(O, null), F(v0.nil), O;
    }
    function F(O = X.async ? v0._`await ` : v0.nil) {
      let A = B.opts.passContext ? Z6.default.this : Z6.default.self, R = !("compile" in X && !H || X.schema === false);
      $.assign(U, v0._`${O}${(0, jA.callValidateCode)(Q, K, A, R)}`, X.modifying);
    }
    function M(O) {
      var A;
      $.if((0, v0.not)((A = X.valid) !== null && A !== void 0 ? A : U), O);
    }
  }
  UH.funcKeywordCode = bA;
  function VH(Q) {
    let { gen: X, data: Y, it: $ } = Q;
    X.if($.parentData, () => X.assign(Y, v0._`${$.parentData}[${$.parentDataProperty}]`));
  }
  function EA(Q, X) {
    let { gen: Y } = Q;
    Y.if(v0._`Array.isArray(${X})`, () => {
      Y.assign(Z6.default.vErrors, v0._`${Z6.default.vErrors} === null ? ${X} : ${Z6.default.vErrors}.concat(${X})`).assign(Z6.default.errors, v0._`${Z6.default.vErrors}.length`), (0, RA.extendErrors)(Q);
    }, () => Q.error());
  }
  function PA({ schemaEnv: Q }, X) {
    if (X.async && !Q.$async) throw Error("async keyword in sync schema");
  }
  function qH(Q, X, Y) {
    if (Y === void 0) throw Error(`keyword "${X}" failed to compile`);
    return Q.scopeValue("keyword", typeof Y == "function" ? { ref: Y } : { ref: Y, code: (0, v0.stringify)(Y) });
  }
  function ZA(Q, X, Y = false) {
    return !X.length || X.some(($) => $ === "array" ? Array.isArray(Q) : $ === "object" ? Q && typeof Q == "object" && !Array.isArray(Q) : typeof Q == $ || Y && typeof Q > "u");
  }
  UH.validSchemaType = ZA;
  function CA({ schema: Q, opts: X, self: Y, errSchemaPath: $ }, J, W) {
    if (Array.isArray(J.keyword) ? !J.keyword.includes(W) : J.keyword !== W) throw Error("ajv implementation error");
    let G = J.dependencies;
    if (G === null || G === void 0 ? void 0 : G.some((H) => !Object.prototype.hasOwnProperty.call(Q, H))) throw Error(`parent schema must have dependencies of ${W}: ${G.join(",")}`);
    if (J.validateSchema) {
      if (!J.validateSchema(Q[W])) {
        let B = `keyword "${W}" value is invalid at path "${$}": ` + Y.errorsText(J.validateSchema.errors);
        if (X.validateSchema === "log") Y.logger.error(B);
        else throw Error(B);
      }
    }
  }
  UH.validateKeywordUsage = CA;
});
var MH = E((OH) => {
  Object.defineProperty(OH, "__esModule", { value: true });
  OH.extendSubschemaMode = OH.extendSubschemaData = OH.getSubschema = void 0;
  var R1 = c(), NH = a();
  function vA(Q, { keyword: X, schemaProp: Y, schema: $, schemaPath: J, errSchemaPath: W, topSchemaRef: G }) {
    if (X !== void 0 && $ !== void 0) throw Error('both "keyword" and "schema" passed, only one allowed');
    if (X !== void 0) {
      let H = Q.schema[X];
      return Y === void 0 ? { schema: H, schemaPath: R1._`${Q.schemaPath}${(0, R1.getProperty)(X)}`, errSchemaPath: `${Q.errSchemaPath}/${X}` } : { schema: H[Y], schemaPath: R1._`${Q.schemaPath}${(0, R1.getProperty)(X)}${(0, R1.getProperty)(Y)}`, errSchemaPath: `${Q.errSchemaPath}/${X}/${(0, NH.escapeFragment)(Y)}` };
    }
    if ($ !== void 0) {
      if (J === void 0 || W === void 0 || G === void 0) throw Error('"schemaPath", "errSchemaPath" and "topSchemaRef" are required with "schema"');
      return { schema: $, schemaPath: J, topSchemaRef: G, errSchemaPath: W };
    }
    throw Error('either "keyword" or "schema" must be passed');
  }
  OH.getSubschema = vA;
  function TA(Q, X, { dataProp: Y, dataPropType: $, data: J, dataTypes: W, propertyName: G }) {
    if (J !== void 0 && Y !== void 0) throw Error('both "data" and "dataProp" passed, only one allowed');
    let { gen: H } = X;
    if (Y !== void 0) {
      let { errorPath: z, dataPathArr: K, opts: U } = X, q = H.let("data", R1._`${X.data}${(0, R1.getProperty)(Y)}`, true);
      B(q), Q.errorPath = R1.str`${z}${(0, NH.getErrorPath)(Y, $, U.jsPropertySyntax)}`, Q.parentDataProperty = R1._`${Y}`, Q.dataPathArr = [...K, Q.parentDataProperty];
    }
    if (J !== void 0) {
      let z = J instanceof R1.Name ? J : H.let("data", J, true);
      if (B(z), G !== void 0) Q.propertyName = G;
    }
    if (W) Q.dataTypes = W;
    function B(z) {
      Q.data = z, Q.dataLevel = X.dataLevel + 1, Q.dataTypes = [], X.definedProperties = /* @__PURE__ */ new Set(), Q.parentData = X.data, Q.dataNames = [...X.dataNames, z];
    }
  }
  OH.extendSubschemaData = TA;
  function xA(Q, { jtdDiscriminator: X, jtdMetadata: Y, compositeRule: $, createErrors: J, allErrors: W }) {
    if ($ !== void 0) Q.compositeRule = $;
    if (J !== void 0) Q.createErrors = J;
    if (W !== void 0) Q.allErrors = W;
    Q.jtdDiscriminator = X, Q.jtdMetadata = Y;
  }
  OH.extendSubschemaMode = xA;
});
var $7 = E((px, wH) => {
  wH.exports = function Q(X, Y) {
    if (X === Y) return true;
    if (X && Y && typeof X == "object" && typeof Y == "object") {
      if (X.constructor !== Y.constructor) return false;
      var $, J, W;
      if (Array.isArray(X)) {
        if ($ = X.length, $ != Y.length) return false;
        for (J = $; J-- !== 0; ) if (!Q(X[J], Y[J])) return false;
        return true;
      }
      if (X.constructor === RegExp) return X.source === Y.source && X.flags === Y.flags;
      if (X.valueOf !== Object.prototype.valueOf) return X.valueOf() === Y.valueOf();
      if (X.toString !== Object.prototype.toString) return X.toString() === Y.toString();
      if (W = Object.keys(X), $ = W.length, $ !== Object.keys(Y).length) return false;
      for (J = $; J-- !== 0; ) if (!Object.prototype.hasOwnProperty.call(Y, W[J])) return false;
      for (J = $; J-- !== 0; ) {
        var G = W[J];
        if (!Q(X[G], Y[G])) return false;
      }
      return true;
    }
    return X !== X && Y !== Y;
  };
});
var jH = E((dx, AH) => {
  var J6 = AH.exports = function(Q, X, Y) {
    if (typeof X == "function") Y = X, X = {};
    Y = X.cb || Y;
    var $ = typeof Y == "function" ? Y : Y.pre || function() {
    }, J = Y.post || function() {
    };
    zQ(X, $, J, Q, "", Q);
  };
  J6.keywords = { additionalItems: true, items: true, contains: true, additionalProperties: true, propertyNames: true, not: true, if: true, then: true, else: true };
  J6.arrayKeywords = { items: true, allOf: true, anyOf: true, oneOf: true };
  J6.propsKeywords = { $defs: true, definitions: true, properties: true, patternProperties: true, dependencies: true };
  J6.skipKeywords = { default: true, enum: true, const: true, required: true, maximum: true, minimum: true, exclusiveMaximum: true, exclusiveMinimum: true, multipleOf: true, maxLength: true, minLength: true, pattern: true, format: true, maxItems: true, minItems: true, uniqueItems: true, maxProperties: true, minProperties: true };
  function zQ(Q, X, Y, $, J, W, G, H, B, z) {
    if ($ && typeof $ == "object" && !Array.isArray($)) {
      X($, J, W, G, H, B, z);
      for (var K in $) {
        var U = $[K];
        if (Array.isArray(U)) {
          if (K in J6.arrayKeywords) for (var q = 0; q < U.length; q++) zQ(Q, X, Y, U[q], J + "/" + K + "/" + q, W, J, K, $, q);
        } else if (K in J6.propsKeywords) {
          if (U && typeof U == "object") for (var V in U) zQ(Q, X, Y, U[V], J + "/" + K + "/" + hA(V), W, J, K, $, V);
        } else if (K in J6.keywords || Q.allKeys && !(K in J6.skipKeywords)) zQ(Q, X, Y, U, J + "/" + K, W, J, K, $);
      }
      Y($, J, W, G, H, B, z);
    }
  }
  function hA(Q) {
    return Q.replace(/~/g, "~0").replace(/\//g, "~1");
  }
});
var R4 = E((EH) => {
  Object.defineProperty(EH, "__esModule", { value: true });
  EH.getSchemaRefs = EH.resolveUrl = EH.normalizeId = EH._getFullPath = EH.getFullPath = EH.inlineRef = void 0;
  var fA = a(), uA = $7(), mA = jH(), lA = /* @__PURE__ */ new Set(["type", "format", "pattern", "maxLength", "minLength", "maxProperties", "minProperties", "maxItems", "minItems", "maximum", "minimum", "uniqueItems", "multipleOf", "required", "enum", "const"]);
  function cA(Q, X = true) {
    if (typeof Q == "boolean") return true;
    if (X === true) return !J7(Q);
    if (!X) return false;
    return RH(Q) <= X;
  }
  EH.inlineRef = cA;
  var pA = /* @__PURE__ */ new Set(["$ref", "$recursiveRef", "$recursiveAnchor", "$dynamicRef", "$dynamicAnchor"]);
  function J7(Q) {
    for (let X in Q) {
      if (pA.has(X)) return true;
      let Y = Q[X];
      if (Array.isArray(Y) && Y.some(J7)) return true;
      if (typeof Y == "object" && J7(Y)) return true;
    }
    return false;
  }
  function RH(Q) {
    let X = 0;
    for (let Y in Q) {
      if (Y === "$ref") return 1 / 0;
      if (X++, lA.has(Y)) continue;
      if (typeof Q[Y] == "object") (0, fA.eachItem)(Q[Y], ($) => X += RH($));
      if (X === 1 / 0) return 1 / 0;
    }
    return X;
  }
  function IH(Q, X = "", Y) {
    if (Y !== false) X = I9(X);
    let $ = Q.parse(X);
    return bH(Q, $);
  }
  EH.getFullPath = IH;
  function bH(Q, X) {
    return Q.serialize(X).split("#")[0] + "#";
  }
  EH._getFullPath = bH;
  var dA = /#\/?$/;
  function I9(Q) {
    return Q ? Q.replace(dA, "") : "";
  }
  EH.normalizeId = I9;
  function iA(Q, X, Y) {
    return Y = I9(Y), Q.resolve(X, Y);
  }
  EH.resolveUrl = iA;
  var nA = /^[a-z_][-a-z0-9._]*$/i;
  function oA(Q, X) {
    if (typeof Q == "boolean") return {};
    let { schemaId: Y, uriResolver: $ } = this.opts, J = I9(Q[Y] || X), W = { "": J }, G = IH($, J, false), H = {}, B = /* @__PURE__ */ new Set();
    return mA(Q, { allKeys: true }, (U, q, V, L) => {
      if (L === void 0) return;
      let F = G + q, M = W[L];
      if (typeof U[Y] == "string") M = O.call(this, U[Y]);
      A.call(this, U.$anchor), A.call(this, U.$dynamicAnchor), W[q] = M;
      function O(R) {
        let Z = this.opts.uriResolver.resolve;
        if (R = I9(M ? Z(M, R) : R), B.has(R)) throw K(R);
        B.add(R);
        let C = this.refs[R];
        if (typeof C == "string") C = this.refs[C];
        if (typeof C == "object") z(U, C.schema, R);
        else if (R !== I9(F)) if (R[0] === "#") z(U, H[R], R), H[R] = U;
        else this.refs[R] = F;
        return R;
      }
      function A(R) {
        if (typeof R == "string") {
          if (!nA.test(R)) throw Error(`invalid anchor "${R}"`);
          O.call(this, `#${R}`);
        }
      }
    }), H;
    function z(U, q, V) {
      if (q !== void 0 && !uA(U, q)) throw K(V);
    }
    function K(U) {
      return Error(`reference "${U}" resolves to more than one schema`);
    }
  }
  EH.getSchemaRefs = oA;
});
var E4 = E((lH) => {
  Object.defineProperty(lH, "__esModule", { value: true });
  lH.getData = lH.KeywordCxt = lH.validateFunctionCode = void 0;
  var kH = d3(), ZH = j4(), G7 = t$(), KQ = j4(), Qj = GH(), b4 = FH(), W7 = MH(), v = c(), f = k1(), Xj = R4(), v1 = a(), I4 = A4();
  function Yj(Q) {
    if (xH(Q)) {
      if (yH(Q), TH(Q)) {
        Wj(Q);
        return;
      }
    }
    vH(Q, () => (0, kH.topBoolOrEmptySchema)(Q));
  }
  lH.validateFunctionCode = Yj;
  function vH({ gen: Q, validateName: X, schema: Y, schemaEnv: $, opts: J }, W) {
    if (J.code.es5) Q.func(X, v._`${f.default.data}, ${f.default.valCxt}`, $.$async, () => {
      Q.code(v._`"use strict"; ${CH(Y, J)}`), Jj(Q, J), Q.code(W);
    });
    else Q.func(X, v._`${f.default.data}, ${$j(J)}`, $.$async, () => Q.code(CH(Y, J)).code(W));
  }
  function $j(Q) {
    return v._`{${f.default.instancePath}="", ${f.default.parentData}, ${f.default.parentDataProperty}, ${f.default.rootData}=${f.default.data}${Q.dynamicRef ? v._`, ${f.default.dynamicAnchors}={}` : v.nil}}={}`;
  }
  function Jj(Q, X) {
    Q.if(f.default.valCxt, () => {
      if (Q.var(f.default.instancePath, v._`${f.default.valCxt}.${f.default.instancePath}`), Q.var(f.default.parentData, v._`${f.default.valCxt}.${f.default.parentData}`), Q.var(f.default.parentDataProperty, v._`${f.default.valCxt}.${f.default.parentDataProperty}`), Q.var(f.default.rootData, v._`${f.default.valCxt}.${f.default.rootData}`), X.dynamicRef) Q.var(f.default.dynamicAnchors, v._`${f.default.valCxt}.${f.default.dynamicAnchors}`);
    }, () => {
      if (Q.var(f.default.instancePath, v._`""`), Q.var(f.default.parentData, v._`undefined`), Q.var(f.default.parentDataProperty, v._`undefined`), Q.var(f.default.rootData, f.default.data), X.dynamicRef) Q.var(f.default.dynamicAnchors, v._`{}`);
    });
  }
  function Wj(Q) {
    let { schema: X, opts: Y, gen: $ } = Q;
    vH(Q, () => {
      if (Y.$comment && X.$comment) hH(Q);
      if (Kj(Q), $.let(f.default.vErrors, null), $.let(f.default.errors, 0), Y.unevaluated) Gj(Q);
      gH(Q), Uj(Q);
    });
    return;
  }
  function Gj(Q) {
    let { gen: X, validateName: Y } = Q;
    Q.evaluated = X.const("evaluated", v._`${Y}.evaluated`), X.if(v._`${Q.evaluated}.dynamicProps`, () => X.assign(v._`${Q.evaluated}.props`, v._`undefined`)), X.if(v._`${Q.evaluated}.dynamicItems`, () => X.assign(v._`${Q.evaluated}.items`, v._`undefined`));
  }
  function CH(Q, X) {
    let Y = typeof Q == "object" && Q[X.schemaId];
    return Y && (X.code.source || X.code.process) ? v._`/*# sourceURL=${Y} */` : v.nil;
  }
  function Hj(Q, X) {
    if (xH(Q)) {
      if (yH(Q), TH(Q)) {
        Bj(Q, X);
        return;
      }
    }
    (0, kH.boolOrEmptySchema)(Q, X);
  }
  function TH({ schema: Q, self: X }) {
    if (typeof Q == "boolean") return !Q;
    for (let Y in Q) if (X.RULES.all[Y]) return true;
    return false;
  }
  function xH(Q) {
    return typeof Q.schema != "boolean";
  }
  function Bj(Q, X) {
    let { schema: Y, gen: $, opts: J } = Q;
    if (J.$comment && Y.$comment) hH(Q);
    Vj(Q), qj(Q);
    let W = $.const("_errs", f.default.errors);
    gH(Q, W), $.var(X, v._`${W} === ${f.default.errors}`);
  }
  function yH(Q) {
    (0, v1.checkUnknownRules)(Q), zj(Q);
  }
  function gH(Q, X) {
    if (Q.opts.jtd) return SH(Q, [], false, X);
    let Y = (0, ZH.getSchemaTypes)(Q.schema), $ = (0, ZH.coerceAndCheckDataType)(Q, Y);
    SH(Q, Y, !$, X);
  }
  function zj(Q) {
    let { schema: X, errSchemaPath: Y, opts: $, self: J } = Q;
    if (X.$ref && $.ignoreKeywordsWithRef && (0, v1.schemaHasRulesButRef)(X, J.RULES)) J.logger.warn(`$ref: keywords ignored in schema at path "${Y}"`);
  }
  function Kj(Q) {
    let { schema: X, opts: Y } = Q;
    if (X.default !== void 0 && Y.useDefaults && Y.strictSchema) (0, v1.checkStrictMode)(Q, "default is ignored in the schema root");
  }
  function Vj(Q) {
    let X = Q.schema[Q.opts.schemaId];
    if (X) Q.baseId = (0, Xj.resolveUrl)(Q.opts.uriResolver, Q.baseId, X);
  }
  function qj(Q) {
    if (Q.schema.$async && !Q.schemaEnv.$async) throw Error("async schema in sync schema");
  }
  function hH({ gen: Q, schemaEnv: X, schema: Y, errSchemaPath: $, opts: J }) {
    let W = Y.$comment;
    if (J.$comment === true) Q.code(v._`${f.default.self}.logger.log(${W})`);
    else if (typeof J.$comment == "function") {
      let G = v.str`${$}/$comment`, H = Q.scopeValue("root", { ref: X.root });
      Q.code(v._`${f.default.self}.opts.$comment(${W}, ${G}, ${H}.schema)`);
    }
  }
  function Uj(Q) {
    let { gen: X, schemaEnv: Y, validateName: $, ValidationError: J, opts: W } = Q;
    if (Y.$async) X.if(v._`${f.default.errors} === 0`, () => X.return(f.default.data), () => X.throw(v._`new ${J}(${f.default.vErrors})`));
    else {
      if (X.assign(v._`${$}.errors`, f.default.vErrors), W.unevaluated) Lj(Q);
      X.return(v._`${f.default.errors} === 0`);
    }
  }
  function Lj({ gen: Q, evaluated: X, props: Y, items: $ }) {
    if (Y instanceof v.Name) Q.assign(v._`${X}.props`, Y);
    if ($ instanceof v.Name) Q.assign(v._`${X}.items`, $);
  }
  function SH(Q, X, Y, $) {
    let { gen: J, schema: W, data: G, allErrors: H, opts: B, self: z } = Q, { RULES: K } = z;
    if (W.$ref && (B.ignoreKeywordsWithRef || !(0, v1.schemaHasRulesButRef)(W, K))) {
      J.block(() => uH(Q, "$ref", K.all.$ref.definition));
      return;
    }
    if (!B.jtd) Fj(Q, X);
    J.block(() => {
      for (let q of K.rules) U(q);
      U(K.post);
    });
    function U(q) {
      if (!(0, G7.shouldUseGroup)(W, q)) return;
      if (q.type) {
        if (J.if((0, KQ.checkDataType)(q.type, G, B.strictNumbers)), _H(Q, q), X.length === 1 && X[0] === q.type && Y) J.else(), (0, KQ.reportTypeError)(Q);
        J.endIf();
      } else _H(Q, q);
      if (!H) J.if(v._`${f.default.errors} === ${$ || 0}`);
    }
  }
  function _H(Q, X) {
    let { gen: Y, schema: $, opts: { useDefaults: J } } = Q;
    if (J) (0, Qj.assignDefaults)(Q, X.type);
    Y.block(() => {
      for (let W of X.rules) if ((0, G7.shouldUseRule)($, W)) uH(Q, W.keyword, W.definition, X.type);
    });
  }
  function Fj(Q, X) {
    if (Q.schemaEnv.meta || !Q.opts.strictTypes) return;
    if (Nj(Q, X), !Q.opts.allowUnionTypes) Oj(Q, X);
    Dj(Q, Q.dataTypes);
  }
  function Nj(Q, X) {
    if (!X.length) return;
    if (!Q.dataTypes.length) {
      Q.dataTypes = X;
      return;
    }
    X.forEach((Y) => {
      if (!fH(Q.dataTypes, Y)) H7(Q, `type "${Y}" not allowed by context "${Q.dataTypes.join(",")}"`);
    }), wj(Q, X);
  }
  function Oj(Q, X) {
    if (X.length > 1 && !(X.length === 2 && X.includes("null"))) H7(Q, "use allowUnionTypes to allow union type keyword");
  }
  function Dj(Q, X) {
    let Y = Q.self.RULES.all;
    for (let $ in Y) {
      let J = Y[$];
      if (typeof J == "object" && (0, G7.shouldUseRule)(Q.schema, J)) {
        let { type: W } = J.definition;
        if (W.length && !W.some((G) => Mj(X, G))) H7(Q, `missing type "${W.join(",")}" for keyword "${$}"`);
      }
    }
  }
  function Mj(Q, X) {
    return Q.includes(X) || X === "number" && Q.includes("integer");
  }
  function fH(Q, X) {
    return Q.includes(X) || X === "integer" && Q.includes("number");
  }
  function wj(Q, X) {
    let Y = [];
    for (let $ of Q.dataTypes) if (fH(X, $)) Y.push($);
    else if (X.includes("integer") && $ === "number") Y.push("integer");
    Q.dataTypes = Y;
  }
  function H7(Q, X) {
    let Y = Q.schemaEnv.baseId + Q.errSchemaPath;
    X += ` at "${Y}" (strictTypes)`, (0, v1.checkStrictMode)(Q, X, Q.opts.strictTypes);
  }
  class B7 {
    constructor(Q, X, Y) {
      if ((0, b4.validateKeywordUsage)(Q, X, Y), this.gen = Q.gen, this.allErrors = Q.allErrors, this.keyword = Y, this.data = Q.data, this.schema = Q.schema[Y], this.$data = X.$data && Q.opts.$data && this.schema && this.schema.$data, this.schemaValue = (0, v1.schemaRefOrVal)(Q, this.schema, Y, this.$data), this.schemaType = X.schemaType, this.parentSchema = Q.schema, this.params = {}, this.it = Q, this.def = X, this.$data) this.schemaCode = Q.gen.const("vSchema", mH(this.$data, Q));
      else if (this.schemaCode = this.schemaValue, !(0, b4.validSchemaType)(this.schema, X.schemaType, X.allowUndefined)) throw Error(`${Y} value must be ${JSON.stringify(X.schemaType)}`);
      if ("code" in X ? X.trackErrors : X.errors !== false) this.errsCount = Q.gen.const("_errs", f.default.errors);
    }
    result(Q, X, Y) {
      this.failResult((0, v.not)(Q), X, Y);
    }
    failResult(Q, X, Y) {
      if (this.gen.if(Q), Y) Y();
      else this.error();
      if (X) {
        if (this.gen.else(), X(), this.allErrors) this.gen.endIf();
      } else if (this.allErrors) this.gen.endIf();
      else this.gen.else();
    }
    pass(Q, X) {
      this.failResult((0, v.not)(Q), void 0, X);
    }
    fail(Q) {
      if (Q === void 0) {
        if (this.error(), !this.allErrors) this.gen.if(false);
        return;
      }
      if (this.gen.if(Q), this.error(), this.allErrors) this.gen.endIf();
      else this.gen.else();
    }
    fail$data(Q) {
      if (!this.$data) return this.fail(Q);
      let { schemaCode: X } = this;
      this.fail(v._`${X} !== undefined && (${(0, v.or)(this.invalid$data(), Q)})`);
    }
    error(Q, X, Y) {
      if (X) {
        this.setParams(X), this._error(Q, Y), this.setParams({});
        return;
      }
      this._error(Q, Y);
    }
    _error(Q, X) {
      (Q ? I4.reportExtraError : I4.reportError)(this, this.def.error, X);
    }
    $dataError() {
      (0, I4.reportError)(this, this.def.$dataError || I4.keyword$DataError);
    }
    reset() {
      if (this.errsCount === void 0) throw Error('add "trackErrors" to keyword definition');
      (0, I4.resetErrorsCount)(this.gen, this.errsCount);
    }
    ok(Q) {
      if (!this.allErrors) this.gen.if(Q);
    }
    setParams(Q, X) {
      if (X) Object.assign(this.params, Q);
      else this.params = Q;
    }
    block$data(Q, X, Y = v.nil) {
      this.gen.block(() => {
        this.check$data(Q, Y), X();
      });
    }
    check$data(Q = v.nil, X = v.nil) {
      if (!this.$data) return;
      let { gen: Y, schemaCode: $, schemaType: J, def: W } = this;
      if (Y.if((0, v.or)(v._`${$} === undefined`, X)), Q !== v.nil) Y.assign(Q, true);
      if (J.length || W.validateSchema) {
        if (Y.elseIf(this.invalid$data()), this.$dataError(), Q !== v.nil) Y.assign(Q, false);
      }
      Y.else();
    }
    invalid$data() {
      let { gen: Q, schemaCode: X, schemaType: Y, def: $, it: J } = this;
      return (0, v.or)(W(), G());
      function W() {
        if (Y.length) {
          if (!(X instanceof v.Name)) throw Error("ajv implementation error");
          let H = Array.isArray(Y) ? Y : [Y];
          return v._`${(0, KQ.checkDataTypes)(H, X, J.opts.strictNumbers, KQ.DataType.Wrong)}`;
        }
        return v.nil;
      }
      function G() {
        if ($.validateSchema) {
          let H = Q.scopeValue("validate$data", { ref: $.validateSchema });
          return v._`!${H}(${X})`;
        }
        return v.nil;
      }
    }
    subschema(Q, X) {
      let Y = (0, W7.getSubschema)(this.it, Q);
      (0, W7.extendSubschemaData)(Y, this.it, Q), (0, W7.extendSubschemaMode)(Y, Q);
      let $ = { ...this.it, ...Y, items: void 0, props: void 0 };
      return Hj($, X), $;
    }
    mergeEvaluated(Q, X) {
      let { it: Y, gen: $ } = this;
      if (!Y.opts.unevaluated) return;
      if (Y.props !== true && Q.props !== void 0) Y.props = v1.mergeEvaluated.props($, Q.props, Y.props, X);
      if (Y.items !== true && Q.items !== void 0) Y.items = v1.mergeEvaluated.items($, Q.items, Y.items, X);
    }
    mergeValidEvaluated(Q, X) {
      let { it: Y, gen: $ } = this;
      if (Y.opts.unevaluated && (Y.props !== true || Y.items !== true)) return $.if(X, () => this.mergeEvaluated(Q, v.Name)), true;
    }
  }
  lH.KeywordCxt = B7;
  function uH(Q, X, Y, $) {
    let J = new B7(Q, Y, X);
    if ("code" in Y) Y.code(J, $);
    else if (J.$data && Y.validate) (0, b4.funcKeywordCode)(J, Y);
    else if ("macro" in Y) (0, b4.macroKeywordCode)(J, Y);
    else if (Y.compile || Y.validate) (0, b4.funcKeywordCode)(J, Y);
  }
  var Aj = /^\/(?:[^~]|~0|~1)*$/, jj = /^([0-9]+)(#|\/(?:[^~]|~0|~1)*)?$/;
  function mH(Q, { dataLevel: X, dataNames: Y, dataPathArr: $ }) {
    let J, W;
    if (Q === "") return f.default.rootData;
    if (Q[0] === "/") {
      if (!Aj.test(Q)) throw Error(`Invalid JSON-pointer: ${Q}`);
      J = Q, W = f.default.rootData;
    } else {
      let z = jj.exec(Q);
      if (!z) throw Error(`Invalid JSON-pointer: ${Q}`);
      let K = +z[1];
      if (J = z[2], J === "#") {
        if (K >= X) throw Error(B("property/index", K));
        return $[X - K];
      }
      if (K > X) throw Error(B("data", K));
      if (W = Y[X - K], !J) return W;
    }
    let G = W, H = J.split("/");
    for (let z of H) if (z) W = v._`${W}${(0, v.getProperty)((0, v1.unescapeJsonPointer)(z))}`, G = v._`${G} && ${W}`;
    return G;
    function B(z, K) {
      return `Cannot access ${z} ${K} levels up, current level is ${X}`;
    }
  }
  lH.getData = mH;
});
var VQ = E((dH) => {
  Object.defineProperty(dH, "__esModule", { value: true });
  class pH extends Error {
    constructor(Q) {
      super("validation failed");
      this.errors = Q, this.ajv = this.validation = true;
    }
  }
  dH.default = pH;
});
var P4 = E((nH) => {
  Object.defineProperty(nH, "__esModule", { value: true });
  var z7 = R4();
  class iH extends Error {
    constructor(Q, X, Y, $) {
      super($ || `can't resolve reference ${Y} from id ${X}`);
      this.missingRef = (0, z7.resolveUrl)(Q, X, Y), this.missingSchema = (0, z7.normalizeId)((0, z7.getFullPath)(Q, this.missingRef));
    }
  }
  nH.default = iH;
});
var UQ = E((tH) => {
  Object.defineProperty(tH, "__esModule", { value: true });
  tH.resolveSchema = tH.getCompilingSchema = tH.resolveRef = tH.compileSchema = tH.SchemaEnv = void 0;
  var V1 = c(), Pj = VQ(), C6 = k1(), q1 = R4(), oH = a(), Zj = E4();
  class Z4 {
    constructor(Q) {
      var X;
      this.refs = {}, this.dynamicAnchors = {};
      let Y;
      if (typeof Q.schema == "object") Y = Q.schema;
      this.schema = Q.schema, this.schemaId = Q.schemaId, this.root = Q.root || this, this.baseId = (X = Q.baseId) !== null && X !== void 0 ? X : (0, q1.normalizeId)(Y === null || Y === void 0 ? void 0 : Y[Q.schemaId || "$id"]), this.schemaPath = Q.schemaPath, this.localRefs = Q.localRefs, this.meta = Q.meta, this.$async = Y === null || Y === void 0 ? void 0 : Y.$async, this.refs = {};
    }
  }
  tH.SchemaEnv = Z4;
  function V7(Q) {
    let X = rH.call(this, Q);
    if (X) return X;
    let Y = (0, q1.getFullPath)(this.opts.uriResolver, Q.root.baseId), { es5: $, lines: J } = this.opts.code, { ownProperties: W } = this.opts, G = new V1.CodeGen(this.scope, { es5: $, lines: J, ownProperties: W }), H;
    if (Q.$async) H = G.scopeValue("Error", { ref: Pj.default, code: V1._`require("ajv/dist/runtime/validation_error").default` });
    let B = G.scopeName("validate");
    Q.validateName = B;
    let z = { gen: G, allErrors: this.opts.allErrors, data: C6.default.data, parentData: C6.default.parentData, parentDataProperty: C6.default.parentDataProperty, dataNames: [C6.default.data], dataPathArr: [V1.nil], dataLevel: 0, dataTypes: [], definedProperties: /* @__PURE__ */ new Set(), topSchemaRef: G.scopeValue("schema", this.opts.code.source === true ? { ref: Q.schema, code: (0, V1.stringify)(Q.schema) } : { ref: Q.schema }), validateName: B, ValidationError: H, schema: Q.schema, schemaEnv: Q, rootId: Y, baseId: Q.baseId || Y, schemaPath: V1.nil, errSchemaPath: Q.schemaPath || (this.opts.jtd ? "" : "#"), errorPath: V1._`""`, opts: this.opts, self: this }, K;
    try {
      this._compilations.add(Q), (0, Zj.validateFunctionCode)(z), G.optimize(this.opts.code.optimize);
      let U = G.toString();
      if (K = `${G.scopeRefs(C6.default.scope)}return ${U}`, this.opts.code.process) K = this.opts.code.process(K, Q);
      let V = Function(`${C6.default.self}`, `${C6.default.scope}`, K)(this, this.scope.get());
      if (this.scope.value(B, { ref: V }), V.errors = null, V.schema = Q.schema, V.schemaEnv = Q, Q.$async) V.$async = true;
      if (this.opts.code.source === true) V.source = { validateName: B, validateCode: U, scopeValues: G._values };
      if (this.opts.unevaluated) {
        let { props: L, items: F } = z;
        if (V.evaluated = { props: L instanceof V1.Name ? void 0 : L, items: F instanceof V1.Name ? void 0 : F, dynamicProps: L instanceof V1.Name, dynamicItems: F instanceof V1.Name }, V.source) V.source.evaluated = (0, V1.stringify)(V.evaluated);
      }
      return Q.validate = V, Q;
    } catch (U) {
      if (delete Q.validate, delete Q.validateName, K) this.logger.error("Error compiling schema, function code:", K);
      throw U;
    } finally {
      this._compilations.delete(Q);
    }
  }
  tH.compileSchema = V7;
  function Cj(Q, X, Y) {
    var $;
    Y = (0, q1.resolveUrl)(this.opts.uriResolver, X, Y);
    let J = Q.refs[Y];
    if (J) return J;
    let W = kj.call(this, Q, Y);
    if (W === void 0) {
      let G = ($ = Q.localRefs) === null || $ === void 0 ? void 0 : $[Y], { schemaId: H } = this.opts;
      if (G) W = new Z4({ schema: G, schemaId: H, root: Q, baseId: X });
    }
    if (W === void 0) return;
    return Q.refs[Y] = Sj.call(this, W);
  }
  tH.resolveRef = Cj;
  function Sj(Q) {
    if ((0, q1.inlineRef)(Q.schema, this.opts.inlineRefs)) return Q.schema;
    return Q.validate ? Q : V7.call(this, Q);
  }
  function rH(Q) {
    for (let X of this._compilations) if (_j(X, Q)) return X;
  }
  tH.getCompilingSchema = rH;
  function _j(Q, X) {
    return Q.schema === X.schema && Q.root === X.root && Q.baseId === X.baseId;
  }
  function kj(Q, X) {
    let Y;
    while (typeof (Y = this.refs[X]) == "string") X = Y;
    return Y || this.schemas[X] || qQ.call(this, Q, X);
  }
  function qQ(Q, X) {
    let Y = this.opts.uriResolver.parse(X), $ = (0, q1._getFullPath)(this.opts.uriResolver, Y), J = (0, q1.getFullPath)(this.opts.uriResolver, Q.baseId, void 0);
    if (Object.keys(Q.schema).length > 0 && $ === J) return K7.call(this, Y, Q);
    let W = (0, q1.normalizeId)($), G = this.refs[W] || this.schemas[W];
    if (typeof G == "string") {
      let H = qQ.call(this, Q, G);
      if (typeof (H === null || H === void 0 ? void 0 : H.schema) !== "object") return;
      return K7.call(this, Y, H);
    }
    if (typeof (G === null || G === void 0 ? void 0 : G.schema) !== "object") return;
    if (!G.validate) V7.call(this, G);
    if (W === (0, q1.normalizeId)(X)) {
      let { schema: H } = G, { schemaId: B } = this.opts, z = H[B];
      if (z) J = (0, q1.resolveUrl)(this.opts.uriResolver, J, z);
      return new Z4({ schema: H, schemaId: B, root: Q, baseId: J });
    }
    return K7.call(this, Y, G);
  }
  tH.resolveSchema = qQ;
  var vj = /* @__PURE__ */ new Set(["properties", "patternProperties", "enum", "dependencies", "definitions"]);
  function K7(Q, { baseId: X, schema: Y, root: $ }) {
    var J;
    if (((J = Q.fragment) === null || J === void 0 ? void 0 : J[0]) !== "/") return;
    for (let H of Q.fragment.slice(1).split("/")) {
      if (typeof Y === "boolean") return;
      let B = Y[(0, oH.unescapeFragment)(H)];
      if (B === void 0) return;
      Y = B;
      let z = typeof Y === "object" && Y[this.opts.schemaId];
      if (!vj.has(H) && z) X = (0, q1.resolveUrl)(this.opts.uriResolver, X, z);
    }
    let W;
    if (typeof Y != "boolean" && Y.$ref && !(0, oH.schemaHasRulesButRef)(Y, this.RULES)) {
      let H = (0, q1.resolveUrl)(this.opts.uriResolver, X, Y.$ref);
      W = qQ.call(this, $, H);
    }
    let { schemaId: G } = this.opts;
    if (W = W || new Z4({ schema: Y, schemaId: G, root: $, baseId: X }), W.schema !== W.root.schema) return W;
    return;
  }
});
var sH = E((ax, hj) => {
  hj.exports = { $id: "https://raw.githubusercontent.com/ajv-validator/ajv/master/lib/refs/data.json#", description: "Meta-schema for $data reference (JSON AnySchema extension proposal)", type: "object", required: ["$data"], properties: { $data: { type: "string", anyOf: [{ format: "relative-json-pointer" }, { format: "json-pointer" }] } }, additionalProperties: false };
});
var QB = E((sx, eH) => {
  var fj = { 0: 0, 1: 1, 2: 2, 3: 3, 4: 4, 5: 5, 6: 6, 7: 7, 8: 8, 9: 9, a: 10, A: 10, b: 11, B: 11, c: 12, C: 12, d: 13, D: 13, e: 14, E: 14, f: 15, F: 15 };
  eH.exports = { HEX: fj };
});
var BB = E((ex, HB) => {
  var { HEX: uj } = QB(), mj = /^(?:(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d{2}|[1-9]\d|\d)$/u;
  function JB(Q) {
    if (GB(Q, ".") < 3) return { host: Q, isIPV4: false };
    let X = Q.match(mj) || [], [Y] = X;
    if (Y) return { host: cj(Y, "."), isIPV4: true };
    else return { host: Q, isIPV4: false };
  }
  function q7(Q, X = false) {
    let Y = "", $ = true;
    for (let J of Q) {
      if (uj[J] === void 0) return;
      if (J !== "0" && $ === true) $ = false;
      if (!$) Y += J;
    }
    if (X && Y.length === 0) Y = "0";
    return Y;
  }
  function lj(Q) {
    let X = 0, Y = { error: false, address: "", zone: "" }, $ = [], J = [], W = false, G = false, H = false;
    function B() {
      if (J.length) {
        if (W === false) {
          let z = q7(J);
          if (z !== void 0) $.push(z);
          else return Y.error = true, false;
        }
        J.length = 0;
      }
      return true;
    }
    for (let z = 0; z < Q.length; z++) {
      let K = Q[z];
      if (K === "[" || K === "]") continue;
      if (K === ":") {
        if (G === true) H = true;
        if (!B()) break;
        if (X++, $.push(":"), X > 7) {
          Y.error = true;
          break;
        }
        if (z - 1 >= 0 && Q[z - 1] === ":") G = true;
        continue;
      } else if (K === "%") {
        if (!B()) break;
        W = true;
      } else {
        J.push(K);
        continue;
      }
    }
    if (J.length) if (W) Y.zone = J.join("");
    else if (H) $.push(J.join(""));
    else $.push(q7(J));
    return Y.address = $.join(""), Y;
  }
  function WB(Q) {
    if (GB(Q, ":") < 2) return { host: Q, isIPV6: false };
    let X = lj(Q);
    if (!X.error) {
      let { address: Y, address: $ } = X;
      if (X.zone) Y += "%" + X.zone, $ += "%25" + X.zone;
      return { host: Y, escapedHost: $, isIPV6: true };
    } else return { host: Q, isIPV6: false };
  }
  function cj(Q, X) {
    let Y = "", $ = true, J = Q.length;
    for (let W = 0; W < J; W++) {
      let G = Q[W];
      if (G === "0" && $) {
        if (W + 1 <= J && Q[W + 1] === X || W + 1 === J) Y += G, $ = false;
      } else {
        if (G === X) $ = true;
        else $ = false;
        Y += G;
      }
    }
    return Y;
  }
  function GB(Q, X) {
    let Y = 0;
    for (let $ = 0; $ < Q.length; $++) if (Q[$] === X) Y++;
    return Y;
  }
  var XB = /^\.\.?\//u, YB = /^\/\.(?:\/|$)/u, $B = /^\/\.\.(?:\/|$)/u, pj = /^\/?(?:.|\n)*?(?=\/|$)/u;
  function dj(Q) {
    let X = [];
    while (Q.length) if (Q.match(XB)) Q = Q.replace(XB, "");
    else if (Q.match(YB)) Q = Q.replace(YB, "/");
    else if (Q.match($B)) Q = Q.replace($B, "/"), X.pop();
    else if (Q === "." || Q === "..") Q = "";
    else {
      let Y = Q.match(pj);
      if (Y) {
        let $ = Y[0];
        Q = Q.slice($.length), X.push($);
      } else throw Error("Unexpected dot segment condition");
    }
    return X.join("");
  }
  function ij(Q, X) {
    let Y = X !== true ? escape : unescape;
    if (Q.scheme !== void 0) Q.scheme = Y(Q.scheme);
    if (Q.userinfo !== void 0) Q.userinfo = Y(Q.userinfo);
    if (Q.host !== void 0) Q.host = Y(Q.host);
    if (Q.path !== void 0) Q.path = Y(Q.path);
    if (Q.query !== void 0) Q.query = Y(Q.query);
    if (Q.fragment !== void 0) Q.fragment = Y(Q.fragment);
    return Q;
  }
  function nj(Q) {
    let X = [];
    if (Q.userinfo !== void 0) X.push(Q.userinfo), X.push("@");
    if (Q.host !== void 0) {
      let Y = unescape(Q.host), $ = JB(Y);
      if ($.isIPV4) Y = $.host;
      else {
        let J = WB($.host);
        if (J.isIPV6 === true) Y = `[${J.escapedHost}]`;
        else Y = Q.host;
      }
      X.push(Y);
    }
    if (typeof Q.port === "number" || typeof Q.port === "string") X.push(":"), X.push(String(Q.port));
    return X.length ? X.join("") : void 0;
  }
  HB.exports = { recomposeAuthority: nj, normalizeComponentEncoding: ij, removeDotSegments: dj, normalizeIPv4: JB, normalizeIPv6: WB, stringArrayToHexStripped: q7 };
});
var LB = E((Qy, UB) => {
  var oj = /^[\da-f]{8}-[\da-f]{4}-[\da-f]{4}-[\da-f]{4}-[\da-f]{12}$/iu, rj = /([\da-z][\d\-a-z]{0,31}):((?:[\w!$'()*+,\-.:;=@]|%[\da-f]{2})+)/iu;
  function zB(Q) {
    return typeof Q.secure === "boolean" ? Q.secure : String(Q.scheme).toLowerCase() === "wss";
  }
  function KB(Q) {
    if (!Q.host) Q.error = Q.error || "HTTP URIs must have a host.";
    return Q;
  }
  function VB(Q) {
    let X = String(Q.scheme).toLowerCase() === "https";
    if (Q.port === (X ? 443 : 80) || Q.port === "") Q.port = void 0;
    if (!Q.path) Q.path = "/";
    return Q;
  }
  function tj(Q) {
    return Q.secure = zB(Q), Q.resourceName = (Q.path || "/") + (Q.query ? "?" + Q.query : ""), Q.path = void 0, Q.query = void 0, Q;
  }
  function aj(Q) {
    if (Q.port === (zB(Q) ? 443 : 80) || Q.port === "") Q.port = void 0;
    if (typeof Q.secure === "boolean") Q.scheme = Q.secure ? "wss" : "ws", Q.secure = void 0;
    if (Q.resourceName) {
      let [X, Y] = Q.resourceName.split("?");
      Q.path = X && X !== "/" ? X : void 0, Q.query = Y, Q.resourceName = void 0;
    }
    return Q.fragment = void 0, Q;
  }
  function sj(Q, X) {
    if (!Q.path) return Q.error = "URN can not be parsed", Q;
    let Y = Q.path.match(rj);
    if (Y) {
      let $ = X.scheme || Q.scheme || "urn";
      Q.nid = Y[1].toLowerCase(), Q.nss = Y[2];
      let J = `${$}:${X.nid || Q.nid}`, W = U7[J];
      if (Q.path = void 0, W) Q = W.parse(Q, X);
    } else Q.error = Q.error || "URN can not be parsed.";
    return Q;
  }
  function ej(Q, X) {
    let Y = X.scheme || Q.scheme || "urn", $ = Q.nid.toLowerCase(), J = `${Y}:${X.nid || $}`, W = U7[J];
    if (W) Q = W.serialize(Q, X);
    let G = Q, H = Q.nss;
    return G.path = `${$ || X.nid}:${H}`, X.skipEscape = true, G;
  }
  function QR(Q, X) {
    let Y = Q;
    if (Y.uuid = Y.nss, Y.nss = void 0, !X.tolerant && (!Y.uuid || !oj.test(Y.uuid))) Y.error = Y.error || "UUID is not valid.";
    return Y;
  }
  function XR(Q) {
    let X = Q;
    return X.nss = (Q.uuid || "").toLowerCase(), X;
  }
  var qB = { scheme: "http", domainHost: true, parse: KB, serialize: VB }, YR = { scheme: "https", domainHost: qB.domainHost, parse: KB, serialize: VB }, LQ = { scheme: "ws", domainHost: true, parse: tj, serialize: aj }, $R = { scheme: "wss", domainHost: LQ.domainHost, parse: LQ.parse, serialize: LQ.serialize }, JR = { scheme: "urn", parse: sj, serialize: ej, skipNormalize: true }, WR = { scheme: "urn:uuid", parse: QR, serialize: XR, skipNormalize: true }, U7 = { http: qB, https: YR, ws: LQ, wss: $R, urn: JR, "urn:uuid": WR };
  UB.exports = U7;
});
var NB = E((Xy, NQ) => {
  var { normalizeIPv6: GR, normalizeIPv4: HR, removeDotSegments: C4, recomposeAuthority: BR, normalizeComponentEncoding: FQ } = BB(), L7 = LB();
  function zR(Q, X) {
    if (typeof Q === "string") Q = I1(T1(Q, X), X);
    else if (typeof Q === "object") Q = T1(I1(Q, X), X);
    return Q;
  }
  function KR(Q, X, Y) {
    let $ = Object.assign({ scheme: "null" }, Y), J = FB(T1(Q, $), T1(X, $), $, true);
    return I1(J, { ...$, skipEscape: true });
  }
  function FB(Q, X, Y, $) {
    let J = {};
    if (!$) Q = T1(I1(Q, Y), Y), X = T1(I1(X, Y), Y);
    if (Y = Y || {}, !Y.tolerant && X.scheme) J.scheme = X.scheme, J.userinfo = X.userinfo, J.host = X.host, J.port = X.port, J.path = C4(X.path || ""), J.query = X.query;
    else {
      if (X.userinfo !== void 0 || X.host !== void 0 || X.port !== void 0) J.userinfo = X.userinfo, J.host = X.host, J.port = X.port, J.path = C4(X.path || ""), J.query = X.query;
      else {
        if (!X.path) if (J.path = Q.path, X.query !== void 0) J.query = X.query;
        else J.query = Q.query;
        else {
          if (X.path.charAt(0) === "/") J.path = C4(X.path);
          else {
            if ((Q.userinfo !== void 0 || Q.host !== void 0 || Q.port !== void 0) && !Q.path) J.path = "/" + X.path;
            else if (!Q.path) J.path = X.path;
            else J.path = Q.path.slice(0, Q.path.lastIndexOf("/") + 1) + X.path;
            J.path = C4(J.path);
          }
          J.query = X.query;
        }
        J.userinfo = Q.userinfo, J.host = Q.host, J.port = Q.port;
      }
      J.scheme = Q.scheme;
    }
    return J.fragment = X.fragment, J;
  }
  function VR(Q, X, Y) {
    if (typeof Q === "string") Q = unescape(Q), Q = I1(FQ(T1(Q, Y), true), { ...Y, skipEscape: true });
    else if (typeof Q === "object") Q = I1(FQ(Q, true), { ...Y, skipEscape: true });
    if (typeof X === "string") X = unescape(X), X = I1(FQ(T1(X, Y), true), { ...Y, skipEscape: true });
    else if (typeof X === "object") X = I1(FQ(X, true), { ...Y, skipEscape: true });
    return Q.toLowerCase() === X.toLowerCase();
  }
  function I1(Q, X) {
    let Y = { host: Q.host, scheme: Q.scheme, userinfo: Q.userinfo, port: Q.port, path: Q.path, query: Q.query, nid: Q.nid, nss: Q.nss, uuid: Q.uuid, fragment: Q.fragment, reference: Q.reference, resourceName: Q.resourceName, secure: Q.secure, error: "" }, $ = Object.assign({}, X), J = [], W = L7[($.scheme || Y.scheme || "").toLowerCase()];
    if (W && W.serialize) W.serialize(Y, $);
    if (Y.path !== void 0) if (!$.skipEscape) {
      if (Y.path = escape(Y.path), Y.scheme !== void 0) Y.path = Y.path.split("%3A").join(":");
    } else Y.path = unescape(Y.path);
    if ($.reference !== "suffix" && Y.scheme) J.push(Y.scheme, ":");
    let G = BR(Y);
    if (G !== void 0) {
      if ($.reference !== "suffix") J.push("//");
      if (J.push(G), Y.path && Y.path.charAt(0) !== "/") J.push("/");
    }
    if (Y.path !== void 0) {
      let H = Y.path;
      if (!$.absolutePath && (!W || !W.absolutePath)) H = C4(H);
      if (G === void 0) H = H.replace(/^\/\//u, "/%2F");
      J.push(H);
    }
    if (Y.query !== void 0) J.push("?", Y.query);
    if (Y.fragment !== void 0) J.push("#", Y.fragment);
    return J.join("");
  }
  var qR = Array.from({ length: 127 }, (Q, X) => /[^!"$&'()*+,\-.;=_`a-z{}~]/u.test(String.fromCharCode(X)));
  function UR(Q) {
    let X = 0;
    for (let Y = 0, $ = Q.length; Y < $; ++Y) if (X = Q.charCodeAt(Y), X > 126 || qR[X]) return true;
    return false;
  }
  var LR = /^(?:([^#/:?]+):)?(?:\/\/((?:([^#/?@]*)@)?(\[[^#/?\]]+\]|[^#/:?]*)(?::(\d*))?))?([^#?]*)(?:\?([^#]*))?(?:#((?:.|[\n\r])*))?/u;
  function T1(Q, X) {
    let Y = Object.assign({}, X), $ = { scheme: void 0, userinfo: void 0, host: "", port: void 0, path: "", query: void 0, fragment: void 0 }, J = Q.indexOf("%") !== -1, W = false;
    if (Y.reference === "suffix") Q = (Y.scheme ? Y.scheme + ":" : "") + "//" + Q;
    let G = Q.match(LR);
    if (G) {
      if ($.scheme = G[1], $.userinfo = G[3], $.host = G[4], $.port = parseInt(G[5], 10), $.path = G[6] || "", $.query = G[7], $.fragment = G[8], isNaN($.port)) $.port = G[5];
      if ($.host) {
        let B = HR($.host);
        if (B.isIPV4 === false) {
          let z = GR(B.host);
          $.host = z.host.toLowerCase(), W = z.isIPV6;
        } else $.host = B.host, W = true;
      }
      if ($.scheme === void 0 && $.userinfo === void 0 && $.host === void 0 && $.port === void 0 && $.query === void 0 && !$.path) $.reference = "same-document";
      else if ($.scheme === void 0) $.reference = "relative";
      else if ($.fragment === void 0) $.reference = "absolute";
      else $.reference = "uri";
      if (Y.reference && Y.reference !== "suffix" && Y.reference !== $.reference) $.error = $.error || "URI is not a " + Y.reference + " reference.";
      let H = L7[(Y.scheme || $.scheme || "").toLowerCase()];
      if (!Y.unicodeSupport && (!H || !H.unicodeSupport)) {
        if ($.host && (Y.domainHost || H && H.domainHost) && W === false && UR($.host)) try {
          $.host = URL.domainToASCII($.host.toLowerCase());
        } catch (B) {
          $.error = $.error || "Host's domain name can not be converted to ASCII: " + B;
        }
      }
      if (!H || H && !H.skipNormalize) {
        if (J && $.scheme !== void 0) $.scheme = unescape($.scheme);
        if (J && $.host !== void 0) $.host = unescape($.host);
        if ($.path) $.path = escape(unescape($.path));
        if ($.fragment) $.fragment = encodeURI(decodeURIComponent($.fragment));
      }
      if (H && H.parse) H.parse($, Y);
    } else $.error = $.error || "URI can not be parsed.";
    return $;
  }
  var F7 = { SCHEMES: L7, normalize: zR, resolve: KR, resolveComponents: FB, equal: VR, serialize: I1, parse: T1 };
  NQ.exports = F7;
  NQ.exports.default = F7;
  NQ.exports.fastUri = F7;
});
var MB = E((DB) => {
  Object.defineProperty(DB, "__esModule", { value: true });
  var OB = NB();
  OB.code = 'require("ajv/dist/runtime/uri").default';
  DB.default = OB;
});
var PB = E((x1) => {
  Object.defineProperty(x1, "__esModule", { value: true });
  x1.CodeGen = x1.Name = x1.nil = x1.stringify = x1.str = x1._ = x1.KeywordCxt = void 0;
  var NR = E4();
  Object.defineProperty(x1, "KeywordCxt", { enumerable: true, get: function() {
    return NR.KeywordCxt;
  } });
  var b9 = c();
  Object.defineProperty(x1, "_", { enumerable: true, get: function() {
    return b9._;
  } });
  Object.defineProperty(x1, "str", { enumerable: true, get: function() {
    return b9.str;
  } });
  Object.defineProperty(x1, "stringify", { enumerable: true, get: function() {
    return b9.stringify;
  } });
  Object.defineProperty(x1, "nil", { enumerable: true, get: function() {
    return b9.nil;
  } });
  Object.defineProperty(x1, "Name", { enumerable: true, get: function() {
    return b9.Name;
  } });
  Object.defineProperty(x1, "CodeGen", { enumerable: true, get: function() {
    return b9.CodeGen;
  } });
  var OR = VQ(), IB = P4(), DR = r$(), S4 = UQ(), MR = c(), _4 = R4(), OQ = j4(), O7 = a(), wB = sH(), wR = MB(), bB = (Q, X) => new RegExp(Q, X);
  bB.code = "new RegExp";
  var AR = ["removeAdditional", "useDefaults", "coerceTypes"], jR = /* @__PURE__ */ new Set(["validate", "serialize", "parse", "wrapper", "root", "schema", "keyword", "pattern", "formats", "validate$data", "func", "obj", "Error"]), RR = { errorDataPath: "", format: "`validateFormats: false` can be used instead.", nullable: '"nullable" keyword is supported by default.', jsonPointers: "Deprecated jsPropertySyntax can be used instead.", extendRefs: "Deprecated ignoreKeywordsWithRef can be used instead.", missingRefs: "Pass empty schema with $id that should be ignored to ajv.addSchema.", processCode: "Use option `code: {process: (code, schemaEnv: object) => string}`", sourceCode: "Use option `code: {source: true}`", strictDefaults: "It is default now, see option `strict`.", strictKeywords: "It is default now, see option `strict`.", uniqueItems: '"uniqueItems" keyword is always validated.', unknownFormats: "Disable strict mode or pass `true` to `ajv.addFormat` (or `formats` option).", cache: "Map is used as cache, schema object as key.", serialize: "Map is used as cache, schema object as key.", ajvErrors: "It is default now." }, IR = { ignoreKeywordsWithRef: "", jsPropertySyntax: "", unicode: '"minLength"/"maxLength" account for unicode characters by default.' }, AB = 200;
  function bR(Q) {
    var X, Y, $, J, W, G, H, B, z, K, U, q, V, L, F, M, O, A, R, Z, C, B0, O0, d0, B6;
    let F1 = Q.strict, z6 = (X = Q.code) === null || X === void 0 ? void 0 : X.optimize, y1 = z6 === true || z6 === void 0 ? 1 : z6 || 0, K6 = ($ = (Y = Q.code) === null || Y === void 0 ? void 0 : Y.regExp) !== null && $ !== void 0 ? $ : bB, h = (J = Q.uriResolver) !== null && J !== void 0 ? J : wR.default;
    return { strictSchema: (G = (W = Q.strictSchema) !== null && W !== void 0 ? W : F1) !== null && G !== void 0 ? G : true, strictNumbers: (B = (H = Q.strictNumbers) !== null && H !== void 0 ? H : F1) !== null && B !== void 0 ? B : true, strictTypes: (K = (z = Q.strictTypes) !== null && z !== void 0 ? z : F1) !== null && K !== void 0 ? K : "log", strictTuples: (q = (U = Q.strictTuples) !== null && U !== void 0 ? U : F1) !== null && q !== void 0 ? q : "log", strictRequired: (L = (V = Q.strictRequired) !== null && V !== void 0 ? V : F1) !== null && L !== void 0 ? L : false, code: Q.code ? { ...Q.code, optimize: y1, regExp: K6 } : { optimize: y1, regExp: K6 }, loopRequired: (F = Q.loopRequired) !== null && F !== void 0 ? F : AB, loopEnum: (M = Q.loopEnum) !== null && M !== void 0 ? M : AB, meta: (O = Q.meta) !== null && O !== void 0 ? O : true, messages: (A = Q.messages) !== null && A !== void 0 ? A : true, inlineRefs: (R = Q.inlineRefs) !== null && R !== void 0 ? R : true, schemaId: (Z = Q.schemaId) !== null && Z !== void 0 ? Z : "$id", addUsedSchema: (C = Q.addUsedSchema) !== null && C !== void 0 ? C : true, validateSchema: (B0 = Q.validateSchema) !== null && B0 !== void 0 ? B0 : true, validateFormats: (O0 = Q.validateFormats) !== null && O0 !== void 0 ? O0 : true, unicodeRegExp: (d0 = Q.unicodeRegExp) !== null && d0 !== void 0 ? d0 : true, int32range: (B6 = Q.int32range) !== null && B6 !== void 0 ? B6 : true, uriResolver: h };
  }
  class DQ {
    constructor(Q = {}) {
      this.schemas = {}, this.refs = {}, this.formats = {}, this._compilations = /* @__PURE__ */ new Set(), this._loading = {}, this._cache = /* @__PURE__ */ new Map(), Q = this.opts = { ...Q, ...bR(Q) };
      let { es5: X, lines: Y } = this.opts.code;
      this.scope = new MR.ValueScope({ scope: {}, prefixes: jR, es5: X, lines: Y }), this.logger = _R(Q.logger);
      let $ = Q.validateFormats;
      if (Q.validateFormats = false, this.RULES = (0, DR.getRules)(), jB.call(this, RR, Q, "NOT SUPPORTED"), jB.call(this, IR, Q, "DEPRECATED", "warn"), this._metaOpts = CR.call(this), Q.formats) PR.call(this);
      if (this._addVocabularies(), this._addDefaultMetaSchema(), Q.keywords) ZR.call(this, Q.keywords);
      if (typeof Q.meta == "object") this.addMetaSchema(Q.meta);
      ER.call(this), Q.validateFormats = $;
    }
    _addVocabularies() {
      this.addKeyword("$async");
    }
    _addDefaultMetaSchema() {
      let { $data: Q, meta: X, schemaId: Y } = this.opts, $ = wB;
      if (Y === "id") $ = { ...wB }, $.id = $.$id, delete $.$id;
      if (X && Q) this.addMetaSchema($, $[Y], false);
    }
    defaultMeta() {
      let { meta: Q, schemaId: X } = this.opts;
      return this.opts.defaultMeta = typeof Q == "object" ? Q[X] || Q : void 0;
    }
    validate(Q, X) {
      let Y;
      if (typeof Q == "string") {
        if (Y = this.getSchema(Q), !Y) throw Error(`no schema with key or ref "${Q}"`);
      } else Y = this.compile(Q);
      let $ = Y(X);
      if (!("$async" in Y)) this.errors = Y.errors;
      return $;
    }
    compile(Q, X) {
      let Y = this._addSchema(Q, X);
      return Y.validate || this._compileSchemaEnv(Y);
    }
    compileAsync(Q, X) {
      if (typeof this.opts.loadSchema != "function") throw Error("options.loadSchema should be a function");
      let { loadSchema: Y } = this.opts;
      return $.call(this, Q, X);
      async function $(z, K) {
        await J.call(this, z.$schema);
        let U = this._addSchema(z, K);
        return U.validate || W.call(this, U);
      }
      async function J(z) {
        if (z && !this.getSchema(z)) await $.call(this, { $ref: z }, true);
      }
      async function W(z) {
        try {
          return this._compileSchemaEnv(z);
        } catch (K) {
          if (!(K instanceof IB.default)) throw K;
          return G.call(this, K), await H.call(this, K.missingSchema), W.call(this, z);
        }
      }
      function G({ missingSchema: z, missingRef: K }) {
        if (this.refs[z]) throw Error(`AnySchema ${z} is loaded but ${K} cannot be resolved`);
      }
      async function H(z) {
        let K = await B.call(this, z);
        if (!this.refs[z]) await J.call(this, K.$schema);
        if (!this.refs[z]) this.addSchema(K, z, X);
      }
      async function B(z) {
        let K = this._loading[z];
        if (K) return K;
        try {
          return await (this._loading[z] = Y(z));
        } finally {
          delete this._loading[z];
        }
      }
    }
    addSchema(Q, X, Y, $ = this.opts.validateSchema) {
      if (Array.isArray(Q)) {
        for (let W of Q) this.addSchema(W, void 0, Y, $);
        return this;
      }
      let J;
      if (typeof Q === "object") {
        let { schemaId: W } = this.opts;
        if (J = Q[W], J !== void 0 && typeof J != "string") throw Error(`schema ${W} must be string`);
      }
      return X = (0, _4.normalizeId)(X || J), this._checkUnique(X), this.schemas[X] = this._addSchema(Q, Y, X, $, true), this;
    }
    addMetaSchema(Q, X, Y = this.opts.validateSchema) {
      return this.addSchema(Q, X, true, Y), this;
    }
    validateSchema(Q, X) {
      if (typeof Q == "boolean") return true;
      let Y;
      if (Y = Q.$schema, Y !== void 0 && typeof Y != "string") throw Error("$schema must be a string");
      if (Y = Y || this.opts.defaultMeta || this.defaultMeta(), !Y) return this.logger.warn("meta-schema not available"), this.errors = null, true;
      let $ = this.validate(Y, Q);
      if (!$ && X) {
        let J = "schema is invalid: " + this.errorsText();
        if (this.opts.validateSchema === "log") this.logger.error(J);
        else throw Error(J);
      }
      return $;
    }
    getSchema(Q) {
      let X;
      while (typeof (X = RB.call(this, Q)) == "string") Q = X;
      if (X === void 0) {
        let { schemaId: Y } = this.opts, $ = new S4.SchemaEnv({ schema: {}, schemaId: Y });
        if (X = S4.resolveSchema.call(this, $, Q), !X) return;
        this.refs[Q] = X;
      }
      return X.validate || this._compileSchemaEnv(X);
    }
    removeSchema(Q) {
      if (Q instanceof RegExp) return this._removeAllSchemas(this.schemas, Q), this._removeAllSchemas(this.refs, Q), this;
      switch (typeof Q) {
        case "undefined":
          return this._removeAllSchemas(this.schemas), this._removeAllSchemas(this.refs), this._cache.clear(), this;
        case "string": {
          let X = RB.call(this, Q);
          if (typeof X == "object") this._cache.delete(X.schema);
          return delete this.schemas[Q], delete this.refs[Q], this;
        }
        case "object": {
          let X = Q;
          this._cache.delete(X);
          let Y = Q[this.opts.schemaId];
          if (Y) Y = (0, _4.normalizeId)(Y), delete this.schemas[Y], delete this.refs[Y];
          return this;
        }
        default:
          throw Error("ajv.removeSchema: invalid parameter");
      }
    }
    addVocabulary(Q) {
      for (let X of Q) this.addKeyword(X);
      return this;
    }
    addKeyword(Q, X) {
      let Y;
      if (typeof Q == "string") {
        if (Y = Q, typeof X == "object") this.logger.warn("these parameters are deprecated, see docs for addKeyword"), X.keyword = Y;
      } else if (typeof Q == "object" && X === void 0) {
        if (X = Q, Y = X.keyword, Array.isArray(Y) && !Y.length) throw Error("addKeywords: keyword must be string or non-empty array");
      } else throw Error("invalid addKeywords parameters");
      if (vR.call(this, Y, X), !X) return (0, O7.eachItem)(Y, (J) => N7.call(this, J)), this;
      xR.call(this, X);
      let $ = { ...X, type: (0, OQ.getJSONTypes)(X.type), schemaType: (0, OQ.getJSONTypes)(X.schemaType) };
      return (0, O7.eachItem)(Y, $.type.length === 0 ? (J) => N7.call(this, J, $) : (J) => $.type.forEach((W) => N7.call(this, J, $, W))), this;
    }
    getKeyword(Q) {
      let X = this.RULES.all[Q];
      return typeof X == "object" ? X.definition : !!X;
    }
    removeKeyword(Q) {
      let { RULES: X } = this;
      delete X.keywords[Q], delete X.all[Q];
      for (let Y of X.rules) {
        let $ = Y.rules.findIndex((J) => J.keyword === Q);
        if ($ >= 0) Y.rules.splice($, 1);
      }
      return this;
    }
    addFormat(Q, X) {
      if (typeof X == "string") X = new RegExp(X);
      return this.formats[Q] = X, this;
    }
    errorsText(Q = this.errors, { separator: X = ", ", dataVar: Y = "data" } = {}) {
      if (!Q || Q.length === 0) return "No errors";
      return Q.map(($) => `${Y}${$.instancePath} ${$.message}`).reduce(($, J) => $ + X + J);
    }
    $dataMetaSchema(Q, X) {
      let Y = this.RULES.all;
      Q = JSON.parse(JSON.stringify(Q));
      for (let $ of X) {
        let J = $.split("/").slice(1), W = Q;
        for (let G of J) W = W[G];
        for (let G in Y) {
          let H = Y[G];
          if (typeof H != "object") continue;
          let { $data: B } = H.definition, z = W[G];
          if (B && z) W[G] = EB(z);
        }
      }
      return Q;
    }
    _removeAllSchemas(Q, X) {
      for (let Y in Q) {
        let $ = Q[Y];
        if (!X || X.test(Y)) {
          if (typeof $ == "string") delete Q[Y];
          else if ($ && !$.meta) this._cache.delete($.schema), delete Q[Y];
        }
      }
    }
    _addSchema(Q, X, Y, $ = this.opts.validateSchema, J = this.opts.addUsedSchema) {
      let W, { schemaId: G } = this.opts;
      if (typeof Q == "object") W = Q[G];
      else if (this.opts.jtd) throw Error("schema must be object");
      else if (typeof Q != "boolean") throw Error("schema must be object or boolean");
      let H = this._cache.get(Q);
      if (H !== void 0) return H;
      Y = (0, _4.normalizeId)(W || Y);
      let B = _4.getSchemaRefs.call(this, Q, Y);
      if (H = new S4.SchemaEnv({ schema: Q, schemaId: G, meta: X, baseId: Y, localRefs: B }), this._cache.set(H.schema, H), J && !Y.startsWith("#")) {
        if (Y) this._checkUnique(Y);
        this.refs[Y] = H;
      }
      if ($) this.validateSchema(Q, true);
      return H;
    }
    _checkUnique(Q) {
      if (this.schemas[Q] || this.refs[Q]) throw Error(`schema with key or id "${Q}" already exists`);
    }
    _compileSchemaEnv(Q) {
      if (Q.meta) this._compileMetaSchema(Q);
      else S4.compileSchema.call(this, Q);
      if (!Q.validate) throw Error("ajv implementation error");
      return Q.validate;
    }
    _compileMetaSchema(Q) {
      let X = this.opts;
      this.opts = this._metaOpts;
      try {
        S4.compileSchema.call(this, Q);
      } finally {
        this.opts = X;
      }
    }
  }
  DQ.ValidationError = OR.default;
  DQ.MissingRefError = IB.default;
  x1.default = DQ;
  function jB(Q, X, Y, $ = "error") {
    for (let J in Q) {
      let W = J;
      if (W in X) this.logger[$](`${Y}: option ${J}. ${Q[W]}`);
    }
  }
  function RB(Q) {
    return Q = (0, _4.normalizeId)(Q), this.schemas[Q] || this.refs[Q];
  }
  function ER() {
    let Q = this.opts.schemas;
    if (!Q) return;
    if (Array.isArray(Q)) this.addSchema(Q);
    else for (let X in Q) this.addSchema(Q[X], X);
  }
  function PR() {
    for (let Q in this.opts.formats) {
      let X = this.opts.formats[Q];
      if (X) this.addFormat(Q, X);
    }
  }
  function ZR(Q) {
    if (Array.isArray(Q)) {
      this.addVocabulary(Q);
      return;
    }
    this.logger.warn("keywords option as map is deprecated, pass array");
    for (let X in Q) {
      let Y = Q[X];
      if (!Y.keyword) Y.keyword = X;
      this.addKeyword(Y);
    }
  }
  function CR() {
    let Q = { ...this.opts };
    for (let X of AR) delete Q[X];
    return Q;
  }
  var SR = { log() {
  }, warn() {
  }, error() {
  } };
  function _R(Q) {
    if (Q === false) return SR;
    if (Q === void 0) return console;
    if (Q.log && Q.warn && Q.error) return Q;
    throw Error("logger must implement log, warn and error methods");
  }
  var kR = /^[a-z_$][a-z0-9_$:-]*$/i;
  function vR(Q, X) {
    let { RULES: Y } = this;
    if ((0, O7.eachItem)(Q, ($) => {
      if (Y.keywords[$]) throw Error(`Keyword ${$} is already defined`);
      if (!kR.test($)) throw Error(`Keyword ${$} has invalid name`);
    }), !X) return;
    if (X.$data && !("code" in X || "validate" in X)) throw Error('$data keyword must have "code" or "validate" function');
  }
  function N7(Q, X, Y) {
    var $;
    let J = X === null || X === void 0 ? void 0 : X.post;
    if (Y && J) throw Error('keyword with "post" flag cannot have "type"');
    let { RULES: W } = this, G = J ? W.post : W.rules.find(({ type: B }) => B === Y);
    if (!G) G = { type: Y, rules: [] }, W.rules.push(G);
    if (W.keywords[Q] = true, !X) return;
    let H = { keyword: Q, definition: { ...X, type: (0, OQ.getJSONTypes)(X.type), schemaType: (0, OQ.getJSONTypes)(X.schemaType) } };
    if (X.before) TR.call(this, G, H, X.before);
    else G.rules.push(H);
    W.all[Q] = H, ($ = X.implements) === null || $ === void 0 || $.forEach((B) => this.addKeyword(B));
  }
  function TR(Q, X, Y) {
    let $ = Q.rules.findIndex((J) => J.keyword === Y);
    if ($ >= 0) Q.rules.splice($, 0, X);
    else Q.rules.push(X), this.logger.warn(`rule ${Y} is not defined`);
  }
  function xR(Q) {
    let { metaSchema: X } = Q;
    if (X === void 0) return;
    if (Q.$data && this.opts.$data) X = EB(X);
    Q.validateSchema = this.compile(X, true);
  }
  var yR = { $ref: "https://raw.githubusercontent.com/ajv-validator/ajv/master/lib/refs/data.json#" };
  function EB(Q) {
    return { anyOf: [Q, yR] };
  }
});
var CB = E((ZB) => {
  Object.defineProperty(ZB, "__esModule", { value: true });
  var fR = { keyword: "id", code() {
    throw Error('NOT SUPPORTED: keyword "id", use "$id" for schema ID');
  } };
  ZB.default = fR;
});
var xB = E((vB) => {
  Object.defineProperty(vB, "__esModule", { value: true });
  vB.callRef = vB.getValidate = void 0;
  var mR = P4(), SB = e0(), f0 = c(), E9 = k1(), _B = UQ(), MQ = a(), lR = { keyword: "$ref", schemaType: "string", code(Q) {
    let { gen: X, schema: Y, it: $ } = Q, { baseId: J, schemaEnv: W, validateName: G, opts: H, self: B } = $, { root: z } = W;
    if ((Y === "#" || Y === "#/") && J === z.baseId) return U();
    let K = _B.resolveRef.call(B, z, J, Y);
    if (K === void 0) throw new mR.default($.opts.uriResolver, J, Y);
    if (K instanceof _B.SchemaEnv) return q(K);
    return V(K);
    function U() {
      if (W === z) return wQ(Q, G, W, W.$async);
      let L = X.scopeValue("root", { ref: z });
      return wQ(Q, f0._`${L}.validate`, z, z.$async);
    }
    function q(L) {
      let F = kB(Q, L);
      wQ(Q, F, L, L.$async);
    }
    function V(L) {
      let F = X.scopeValue("schema", H.code.source === true ? { ref: L, code: (0, f0.stringify)(L) } : { ref: L }), M = X.name("valid"), O = Q.subschema({ schema: L, dataTypes: [], schemaPath: f0.nil, topSchemaRef: F, errSchemaPath: Y }, M);
      Q.mergeEvaluated(O), Q.ok(M);
    }
  } };
  function kB(Q, X) {
    let { gen: Y } = Q;
    return X.validate ? Y.scopeValue("validate", { ref: X.validate }) : f0._`${Y.scopeValue("wrapper", { ref: X })}.validate`;
  }
  vB.getValidate = kB;
  function wQ(Q, X, Y, $) {
    let { gen: J, it: W } = Q, { allErrors: G, schemaEnv: H, opts: B } = W, z = B.passContext ? E9.default.this : f0.nil;
    if ($) K();
    else U();
    function K() {
      if (!H.$async) throw Error("async schema referenced by sync schema");
      let L = J.let("valid");
      J.try(() => {
        if (J.code(f0._`await ${(0, SB.callValidateCode)(Q, X, z)}`), V(X), !G) J.assign(L, true);
      }, (F) => {
        if (J.if(f0._`!(${F} instanceof ${W.ValidationError})`, () => J.throw(F)), q(F), !G) J.assign(L, false);
      }), Q.ok(L);
    }
    function U() {
      Q.result((0, SB.callValidateCode)(Q, X, z), () => V(X), () => q(X));
    }
    function q(L) {
      let F = f0._`${L}.errors`;
      J.assign(E9.default.vErrors, f0._`${E9.default.vErrors} === null ? ${F} : ${E9.default.vErrors}.concat(${F})`), J.assign(E9.default.errors, f0._`${E9.default.vErrors}.length`);
    }
    function V(L) {
      var F;
      if (!W.opts.unevaluated) return;
      let M = (F = Y === null || Y === void 0 ? void 0 : Y.validate) === null || F === void 0 ? void 0 : F.evaluated;
      if (W.props !== true) if (M && !M.dynamicProps) {
        if (M.props !== void 0) W.props = MQ.mergeEvaluated.props(J, M.props, W.props);
      } else {
        let O = J.var("props", f0._`${L}.evaluated.props`);
        W.props = MQ.mergeEvaluated.props(J, O, W.props, f0.Name);
      }
      if (W.items !== true) if (M && !M.dynamicItems) {
        if (M.items !== void 0) W.items = MQ.mergeEvaluated.items(J, M.items, W.items);
      } else {
        let O = J.var("items", f0._`${L}.evaluated.items`);
        W.items = MQ.mergeEvaluated.items(J, O, W.items, f0.Name);
      }
    }
  }
  vB.callRef = wQ;
  vB.default = lR;
});
var gB = E((yB) => {
  Object.defineProperty(yB, "__esModule", { value: true });
  var dR = CB(), iR = xB(), nR = ["$schema", "$id", "$defs", "$vocabulary", { keyword: "$comment" }, "definitions", dR.default, iR.default];
  yB.default = nR;
});
var fB = E((hB) => {
  Object.defineProperty(hB, "__esModule", { value: true });
  var AQ = c(), W6 = AQ.operators, jQ = { maximum: { okStr: "<=", ok: W6.LTE, fail: W6.GT }, minimum: { okStr: ">=", ok: W6.GTE, fail: W6.LT }, exclusiveMaximum: { okStr: "<", ok: W6.LT, fail: W6.GTE }, exclusiveMinimum: { okStr: ">", ok: W6.GT, fail: W6.LTE } }, rR = { message: ({ keyword: Q, schemaCode: X }) => AQ.str`must be ${jQ[Q].okStr} ${X}`, params: ({ keyword: Q, schemaCode: X }) => AQ._`{comparison: ${jQ[Q].okStr}, limit: ${X}}` }, tR = { keyword: Object.keys(jQ), type: "number", schemaType: "number", $data: true, error: rR, code(Q) {
    let { keyword: X, data: Y, schemaCode: $ } = Q;
    Q.fail$data(AQ._`${Y} ${jQ[X].fail} ${$} || isNaN(${Y})`);
  } };
  hB.default = tR;
});
var mB = E((uB) => {
  Object.defineProperty(uB, "__esModule", { value: true });
  var k4 = c(), sR = { message: ({ schemaCode: Q }) => k4.str`must be multiple of ${Q}`, params: ({ schemaCode: Q }) => k4._`{multipleOf: ${Q}}` }, eR = { keyword: "multipleOf", type: "number", schemaType: "number", $data: true, error: sR, code(Q) {
    let { gen: X, data: Y, schemaCode: $, it: J } = Q, W = J.opts.multipleOfPrecision, G = X.let("res"), H = W ? k4._`Math.abs(Math.round(${G}) - ${G}) > 1e-${W}` : k4._`${G} !== parseInt(${G})`;
    Q.fail$data(k4._`(${$} === 0 || (${G} = ${Y}/${$}, ${H}))`);
  } };
  uB.default = eR;
});
var pB = E((cB) => {
  Object.defineProperty(cB, "__esModule", { value: true });
  function lB(Q) {
    let X = Q.length, Y = 0, $ = 0, J;
    while ($ < X) if (Y++, J = Q.charCodeAt($++), J >= 55296 && J <= 56319 && $ < X) {
      if (J = Q.charCodeAt($), (J & 64512) === 56320) $++;
    }
    return Y;
  }
  cB.default = lB;
  lB.code = 'require("ajv/dist/runtime/ucs2length").default';
});
var iB = E((dB) => {
  Object.defineProperty(dB, "__esModule", { value: true });
  var S6 = c(), Y2 = a(), $2 = pB(), J2 = { message({ keyword: Q, schemaCode: X }) {
    let Y = Q === "maxLength" ? "more" : "fewer";
    return S6.str`must NOT have ${Y} than ${X} characters`;
  }, params: ({ schemaCode: Q }) => S6._`{limit: ${Q}}` }, W2 = { keyword: ["maxLength", "minLength"], type: "string", schemaType: "number", $data: true, error: J2, code(Q) {
    let { keyword: X, data: Y, schemaCode: $, it: J } = Q, W = X === "maxLength" ? S6.operators.GT : S6.operators.LT, G = J.opts.unicode === false ? S6._`${Y}.length` : S6._`${(0, Y2.useFunc)(Q.gen, $2.default)}(${Y})`;
    Q.fail$data(S6._`${G} ${W} ${$}`);
  } };
  dB.default = W2;
});
var oB = E((nB) => {
  Object.defineProperty(nB, "__esModule", { value: true });
  var H2 = e0(), RQ = c(), B2 = { message: ({ schemaCode: Q }) => RQ.str`must match pattern "${Q}"`, params: ({ schemaCode: Q }) => RQ._`{pattern: ${Q}}` }, z2 = { keyword: "pattern", type: "string", schemaType: "string", $data: true, error: B2, code(Q) {
    let { data: X, $data: Y, schema: $, schemaCode: J, it: W } = Q, G = W.opts.unicodeRegExp ? "u" : "", H = Y ? RQ._`(new RegExp(${J}, ${G}))` : (0, H2.usePattern)(Q, $);
    Q.fail$data(RQ._`!${H}.test(${X})`);
  } };
  nB.default = z2;
});
var tB = E((rB) => {
  Object.defineProperty(rB, "__esModule", { value: true });
  var v4 = c(), V2 = { message({ keyword: Q, schemaCode: X }) {
    let Y = Q === "maxProperties" ? "more" : "fewer";
    return v4.str`must NOT have ${Y} than ${X} properties`;
  }, params: ({ schemaCode: Q }) => v4._`{limit: ${Q}}` }, q2 = { keyword: ["maxProperties", "minProperties"], type: "object", schemaType: "number", $data: true, error: V2, code(Q) {
    let { keyword: X, data: Y, schemaCode: $ } = Q, J = X === "maxProperties" ? v4.operators.GT : v4.operators.LT;
    Q.fail$data(v4._`Object.keys(${Y}).length ${J} ${$}`);
  } };
  rB.default = q2;
});
var sB = E((aB) => {
  Object.defineProperty(aB, "__esModule", { value: true });
  var T4 = e0(), x4 = c(), L2 = a(), F2 = { message: ({ params: { missingProperty: Q } }) => x4.str`must have required property '${Q}'`, params: ({ params: { missingProperty: Q } }) => x4._`{missingProperty: ${Q}}` }, N2 = { keyword: "required", type: "object", schemaType: "array", $data: true, error: F2, code(Q) {
    let { gen: X, schema: Y, schemaCode: $, data: J, $data: W, it: G } = Q, { opts: H } = G;
    if (!W && Y.length === 0) return;
    let B = Y.length >= H.loopRequired;
    if (G.allErrors) z();
    else K();
    if (H.strictRequired) {
      let V = Q.parentSchema.properties, { definedProperties: L } = Q.it;
      for (let F of Y) if ((V === null || V === void 0 ? void 0 : V[F]) === void 0 && !L.has(F)) {
        let M = G.schemaEnv.baseId + G.errSchemaPath, O = `required property "${F}" is not defined at "${M}" (strictRequired)`;
        (0, L2.checkStrictMode)(G, O, G.opts.strictRequired);
      }
    }
    function z() {
      if (B || W) Q.block$data(x4.nil, U);
      else for (let V of Y) (0, T4.checkReportMissingProp)(Q, V);
    }
    function K() {
      let V = X.let("missing");
      if (B || W) {
        let L = X.let("valid", true);
        Q.block$data(L, () => q(V, L)), Q.ok(L);
      } else X.if((0, T4.checkMissingProp)(Q, Y, V)), (0, T4.reportMissingProp)(Q, V), X.else();
    }
    function U() {
      X.forOf("prop", $, (V) => {
        Q.setParams({ missingProperty: V }), X.if((0, T4.noPropertyInData)(X, J, V, H.ownProperties), () => Q.error());
      });
    }
    function q(V, L) {
      Q.setParams({ missingProperty: V }), X.forOf(V, $, () => {
        X.assign(L, (0, T4.propertyInData)(X, J, V, H.ownProperties)), X.if((0, x4.not)(L), () => {
          Q.error(), X.break();
        });
      }, x4.nil);
    }
  } };
  aB.default = N2;
});
var Qz = E((eB) => {
  Object.defineProperty(eB, "__esModule", { value: true });
  var y4 = c(), D2 = { message({ keyword: Q, schemaCode: X }) {
    let Y = Q === "maxItems" ? "more" : "fewer";
    return y4.str`must NOT have ${Y} than ${X} items`;
  }, params: ({ schemaCode: Q }) => y4._`{limit: ${Q}}` }, M2 = { keyword: ["maxItems", "minItems"], type: "array", schemaType: "number", $data: true, error: D2, code(Q) {
    let { keyword: X, data: Y, schemaCode: $ } = Q, J = X === "maxItems" ? y4.operators.GT : y4.operators.LT;
    Q.fail$data(y4._`${Y}.length ${J} ${$}`);
  } };
  eB.default = M2;
});
var IQ = E((Yz) => {
  Object.defineProperty(Yz, "__esModule", { value: true });
  var Xz = $7();
  Xz.code = 'require("ajv/dist/runtime/equal").default';
  Yz.default = Xz;
});
var Jz = E(($z) => {
  Object.defineProperty($z, "__esModule", { value: true });
  var D7 = j4(), E0 = c(), j2 = a(), R2 = IQ(), I2 = { message: ({ params: { i: Q, j: X } }) => E0.str`must NOT have duplicate items (items ## ${X} and ${Q} are identical)`, params: ({ params: { i: Q, j: X } }) => E0._`{i: ${Q}, j: ${X}}` }, b2 = { keyword: "uniqueItems", type: "array", schemaType: "boolean", $data: true, error: I2, code(Q) {
    let { gen: X, data: Y, $data: $, schema: J, parentSchema: W, schemaCode: G, it: H } = Q;
    if (!$ && !J) return;
    let B = X.let("valid"), z = W.items ? (0, D7.getSchemaTypes)(W.items) : [];
    Q.block$data(B, K, E0._`${G} === false`), Q.ok(B);
    function K() {
      let L = X.let("i", E0._`${Y}.length`), F = X.let("j");
      Q.setParams({ i: L, j: F }), X.assign(B, true), X.if(E0._`${L} > 1`, () => (U() ? q : V)(L, F));
    }
    function U() {
      return z.length > 0 && !z.some((L) => L === "object" || L === "array");
    }
    function q(L, F) {
      let M = X.name("item"), O = (0, D7.checkDataTypes)(z, M, H.opts.strictNumbers, D7.DataType.Wrong), A = X.const("indices", E0._`{}`);
      X.for(E0._`;${L}--;`, () => {
        if (X.let(M, E0._`${Y}[${L}]`), X.if(O, E0._`continue`), z.length > 1) X.if(E0._`typeof ${M} == "string"`, E0._`${M} += "_"`);
        X.if(E0._`typeof ${A}[${M}] == "number"`, () => {
          X.assign(F, E0._`${A}[${M}]`), Q.error(), X.assign(B, false).break();
        }).code(E0._`${A}[${M}] = ${L}`);
      });
    }
    function V(L, F) {
      let M = (0, j2.useFunc)(X, R2.default), O = X.name("outer");
      X.label(O).for(E0._`;${L}--;`, () => X.for(E0._`${F} = ${L}; ${F}--;`, () => X.if(E0._`${M}(${Y}[${L}], ${Y}[${F}])`, () => {
        Q.error(), X.assign(B, false).break(O);
      })));
    }
  } };
  $z.default = b2;
});
var Gz = E((Wz) => {
  Object.defineProperty(Wz, "__esModule", { value: true });
  var M7 = c(), P2 = a(), Z2 = IQ(), C2 = { message: "must be equal to constant", params: ({ schemaCode: Q }) => M7._`{allowedValue: ${Q}}` }, S2 = { keyword: "const", $data: true, error: C2, code(Q) {
    let { gen: X, data: Y, $data: $, schemaCode: J, schema: W } = Q;
    if ($ || W && typeof W == "object") Q.fail$data(M7._`!${(0, P2.useFunc)(X, Z2.default)}(${Y}, ${J})`);
    else Q.fail(M7._`${W} !== ${Y}`);
  } };
  Wz.default = S2;
});
var Bz = E((Hz) => {
  Object.defineProperty(Hz, "__esModule", { value: true });
  var g4 = c(), k2 = a(), v2 = IQ(), T2 = { message: "must be equal to one of the allowed values", params: ({ schemaCode: Q }) => g4._`{allowedValues: ${Q}}` }, x2 = { keyword: "enum", schemaType: "array", $data: true, error: T2, code(Q) {
    let { gen: X, data: Y, $data: $, schema: J, schemaCode: W, it: G } = Q;
    if (!$ && J.length === 0) throw Error("enum must have non-empty array");
    let H = J.length >= G.opts.loopEnum, B, z = () => B !== null && B !== void 0 ? B : B = (0, k2.useFunc)(X, v2.default), K;
    if (H || $) K = X.let("valid"), Q.block$data(K, U);
    else {
      if (!Array.isArray(J)) throw Error("ajv implementation error");
      let V = X.const("vSchema", W);
      K = (0, g4.or)(...J.map((L, F) => q(V, F)));
    }
    Q.pass(K);
    function U() {
      X.assign(K, false), X.forOf("v", W, (V) => X.if(g4._`${z()}(${Y}, ${V})`, () => X.assign(K, true).break()));
    }
    function q(V, L) {
      let F = J[L];
      return typeof F === "object" && F !== null ? g4._`${z()}(${Y}, ${V}[${L}])` : g4._`${Y} === ${F}`;
    }
  } };
  Hz.default = x2;
});
var Kz = E((zz) => {
  Object.defineProperty(zz, "__esModule", { value: true });
  var g2 = fB(), h2 = mB(), f2 = iB(), u2 = oB(), m2 = tB(), l2 = sB(), c2 = Qz(), p2 = Jz(), d2 = Gz(), i2 = Bz(), n2 = [g2.default, h2.default, f2.default, u2.default, m2.default, l2.default, c2.default, p2.default, { keyword: "type", schemaType: ["string", "array"] }, { keyword: "nullable", schemaType: "boolean" }, d2.default, i2.default];
  zz.default = n2;
});
var A7 = E((qz) => {
  Object.defineProperty(qz, "__esModule", { value: true });
  qz.validateAdditionalItems = void 0;
  var _6 = c(), w7 = a(), r2 = { message: ({ params: { len: Q } }) => _6.str`must NOT have more than ${Q} items`, params: ({ params: { len: Q } }) => _6._`{limit: ${Q}}` }, t2 = { keyword: "additionalItems", type: "array", schemaType: ["boolean", "object"], before: "uniqueItems", error: r2, code(Q) {
    let { parentSchema: X, it: Y } = Q, { items: $ } = X;
    if (!Array.isArray($)) {
      (0, w7.checkStrictMode)(Y, '"additionalItems" is ignored when "items" is not an array of schemas');
      return;
    }
    Vz(Q, $);
  } };
  function Vz(Q, X) {
    let { gen: Y, schema: $, data: J, keyword: W, it: G } = Q;
    G.items = true;
    let H = Y.const("len", _6._`${J}.length`);
    if ($ === false) Q.setParams({ len: X.length }), Q.pass(_6._`${H} <= ${X.length}`);
    else if (typeof $ == "object" && !(0, w7.alwaysValidSchema)(G, $)) {
      let z = Y.var("valid", _6._`${H} <= ${X.length}`);
      Y.if((0, _6.not)(z), () => B(z)), Q.ok(z);
    }
    function B(z) {
      Y.forRange("i", X.length, H, (K) => {
        if (Q.subschema({ keyword: W, dataProp: K, dataPropType: w7.Type.Num }, z), !G.allErrors) Y.if((0, _6.not)(z), () => Y.break());
      });
    }
  }
  qz.validateAdditionalItems = Vz;
  qz.default = t2;
});
var j7 = E((Nz) => {
  Object.defineProperty(Nz, "__esModule", { value: true });
  Nz.validateTuple = void 0;
  var Lz = c(), bQ = a(), s2 = e0(), e2 = { keyword: "items", type: "array", schemaType: ["object", "array", "boolean"], before: "uniqueItems", code(Q) {
    let { schema: X, it: Y } = Q;
    if (Array.isArray(X)) return Fz(Q, "additionalItems", X);
    if (Y.items = true, (0, bQ.alwaysValidSchema)(Y, X)) return;
    Q.ok((0, s2.validateArray)(Q));
  } };
  function Fz(Q, X, Y = Q.schema) {
    let { gen: $, parentSchema: J, data: W, keyword: G, it: H } = Q;
    if (K(J), H.opts.unevaluated && Y.length && H.items !== true) H.items = bQ.mergeEvaluated.items($, Y.length, H.items);
    let B = $.name("valid"), z = $.const("len", Lz._`${W}.length`);
    Y.forEach((U, q) => {
      if ((0, bQ.alwaysValidSchema)(H, U)) return;
      $.if(Lz._`${z} > ${q}`, () => Q.subschema({ keyword: G, schemaProp: q, dataProp: q }, B)), Q.ok(B);
    });
    function K(U) {
      let { opts: q, errSchemaPath: V } = H, L = Y.length, F = L === U.minItems && (L === U.maxItems || U[X] === false);
      if (q.strictTuples && !F) {
        let M = `"${G}" is ${L}-tuple, but minItems or maxItems/${X} are not specified or different at path "${V}"`;
        (0, bQ.checkStrictMode)(H, M, q.strictTuples);
      }
    }
  }
  Nz.validateTuple = Fz;
  Nz.default = e2;
});
var Mz = E((Dz) => {
  Object.defineProperty(Dz, "__esModule", { value: true });
  var XI = j7(), YI = { keyword: "prefixItems", type: "array", schemaType: ["array"], before: "uniqueItems", code: (Q) => (0, XI.validateTuple)(Q, "items") };
  Dz.default = YI;
});
var jz = E((Az) => {
  Object.defineProperty(Az, "__esModule", { value: true });
  var wz = c(), JI = a(), WI = e0(), GI = A7(), HI = { message: ({ params: { len: Q } }) => wz.str`must NOT have more than ${Q} items`, params: ({ params: { len: Q } }) => wz._`{limit: ${Q}}` }, BI = { keyword: "items", type: "array", schemaType: ["object", "boolean"], before: "uniqueItems", error: HI, code(Q) {
    let { schema: X, parentSchema: Y, it: $ } = Q, { prefixItems: J } = Y;
    if ($.items = true, (0, JI.alwaysValidSchema)($, X)) return;
    if (J) (0, GI.validateAdditionalItems)(Q, J);
    else Q.ok((0, WI.validateArray)(Q));
  } };
  Az.default = BI;
});
var Iz = E((Rz) => {
  Object.defineProperty(Rz, "__esModule", { value: true });
  var Q1 = c(), EQ = a(), KI = { message: ({ params: { min: Q, max: X } }) => X === void 0 ? Q1.str`must contain at least ${Q} valid item(s)` : Q1.str`must contain at least ${Q} and no more than ${X} valid item(s)`, params: ({ params: { min: Q, max: X } }) => X === void 0 ? Q1._`{minContains: ${Q}}` : Q1._`{minContains: ${Q}, maxContains: ${X}}` }, VI = { keyword: "contains", type: "array", schemaType: ["object", "boolean"], before: "uniqueItems", trackErrors: true, error: KI, code(Q) {
    let { gen: X, schema: Y, parentSchema: $, data: J, it: W } = Q, G, H, { minContains: B, maxContains: z } = $;
    if (W.opts.next) G = B === void 0 ? 1 : B, H = z;
    else G = 1;
    let K = X.const("len", Q1._`${J}.length`);
    if (Q.setParams({ min: G, max: H }), H === void 0 && G === 0) {
      (0, EQ.checkStrictMode)(W, '"minContains" == 0 without "maxContains": "contains" keyword ignored');
      return;
    }
    if (H !== void 0 && G > H) {
      (0, EQ.checkStrictMode)(W, '"minContains" > "maxContains" is always invalid'), Q.fail();
      return;
    }
    if ((0, EQ.alwaysValidSchema)(W, Y)) {
      let F = Q1._`${K} >= ${G}`;
      if (H !== void 0) F = Q1._`${F} && ${K} <= ${H}`;
      Q.pass(F);
      return;
    }
    W.items = true;
    let U = X.name("valid");
    if (H === void 0 && G === 1) V(U, () => X.if(U, () => X.break()));
    else if (G === 0) {
      if (X.let(U, true), H !== void 0) X.if(Q1._`${J}.length > 0`, q);
    } else X.let(U, false), q();
    Q.result(U, () => Q.reset());
    function q() {
      let F = X.name("_valid"), M = X.let("count", 0);
      V(F, () => X.if(F, () => L(M)));
    }
    function V(F, M) {
      X.forRange("i", 0, K, (O) => {
        Q.subschema({ keyword: "contains", dataProp: O, dataPropType: EQ.Type.Num, compositeRule: true }, F), M();
      });
    }
    function L(F) {
      if (X.code(Q1._`${F}++`), H === void 0) X.if(Q1._`${F} >= ${G}`, () => X.assign(U, true).break());
      else if (X.if(Q1._`${F} > ${H}`, () => X.assign(U, false).break()), G === 1) X.assign(U, true);
      else X.if(Q1._`${F} >= ${G}`, () => X.assign(U, true));
    }
  } };
  Rz.default = VI;
});
var Sz = E((Pz) => {
  Object.defineProperty(Pz, "__esModule", { value: true });
  Pz.validateSchemaDeps = Pz.validatePropertyDeps = Pz.error = void 0;
  var R7 = c(), UI = a(), h4 = e0();
  Pz.error = { message: ({ params: { property: Q, depsCount: X, deps: Y } }) => {
    let $ = X === 1 ? "property" : "properties";
    return R7.str`must have ${$} ${Y} when property ${Q} is present`;
  }, params: ({ params: { property: Q, depsCount: X, deps: Y, missingProperty: $ } }) => R7._`{property: ${Q},
    missingProperty: ${$},
    depsCount: ${X},
    deps: ${Y}}` };
  var LI = { keyword: "dependencies", type: "object", schemaType: "object", error: Pz.error, code(Q) {
    let [X, Y] = FI(Q);
    bz(Q, X), Ez(Q, Y);
  } };
  function FI({ schema: Q }) {
    let X = {}, Y = {};
    for (let $ in Q) {
      if ($ === "__proto__") continue;
      let J = Array.isArray(Q[$]) ? X : Y;
      J[$] = Q[$];
    }
    return [X, Y];
  }
  function bz(Q, X = Q.schema) {
    let { gen: Y, data: $, it: J } = Q;
    if (Object.keys(X).length === 0) return;
    let W = Y.let("missing");
    for (let G in X) {
      let H = X[G];
      if (H.length === 0) continue;
      let B = (0, h4.propertyInData)(Y, $, G, J.opts.ownProperties);
      if (Q.setParams({ property: G, depsCount: H.length, deps: H.join(", ") }), J.allErrors) Y.if(B, () => {
        for (let z of H) (0, h4.checkReportMissingProp)(Q, z);
      });
      else Y.if(R7._`${B} && (${(0, h4.checkMissingProp)(Q, H, W)})`), (0, h4.reportMissingProp)(Q, W), Y.else();
    }
  }
  Pz.validatePropertyDeps = bz;
  function Ez(Q, X = Q.schema) {
    let { gen: Y, data: $, keyword: J, it: W } = Q, G = Y.name("valid");
    for (let H in X) {
      if ((0, UI.alwaysValidSchema)(W, X[H])) continue;
      Y.if((0, h4.propertyInData)(Y, $, H, W.opts.ownProperties), () => {
        let B = Q.subschema({ keyword: J, schemaProp: H }, G);
        Q.mergeValidEvaluated(B, G);
      }, () => Y.var(G, true)), Q.ok(G);
    }
  }
  Pz.validateSchemaDeps = Ez;
  Pz.default = LI;
});
var vz = E((kz) => {
  Object.defineProperty(kz, "__esModule", { value: true });
  var _z = c(), DI = a(), MI = { message: "property name must be valid", params: ({ params: Q }) => _z._`{propertyName: ${Q.propertyName}}` }, wI = { keyword: "propertyNames", type: "object", schemaType: ["object", "boolean"], error: MI, code(Q) {
    let { gen: X, schema: Y, data: $, it: J } = Q;
    if ((0, DI.alwaysValidSchema)(J, Y)) return;
    let W = X.name("valid");
    X.forIn("key", $, (G) => {
      Q.setParams({ propertyName: G }), Q.subschema({ keyword: "propertyNames", data: G, dataTypes: ["string"], propertyName: G, compositeRule: true }, W), X.if((0, _z.not)(W), () => {
        if (Q.error(true), !J.allErrors) X.break();
      });
    }), Q.ok(W);
  } };
  kz.default = wI;
});
var I7 = E((Tz) => {
  Object.defineProperty(Tz, "__esModule", { value: true });
  var PQ = e0(), U1 = c(), jI = k1(), ZQ = a(), RI = { message: "must NOT have additional properties", params: ({ params: Q }) => U1._`{additionalProperty: ${Q.additionalProperty}}` }, II = { keyword: "additionalProperties", type: ["object"], schemaType: ["boolean", "object"], allowUndefined: true, trackErrors: true, error: RI, code(Q) {
    let { gen: X, schema: Y, parentSchema: $, data: J, errsCount: W, it: G } = Q;
    if (!W) throw Error("ajv implementation error");
    let { allErrors: H, opts: B } = G;
    if (G.props = true, B.removeAdditional !== "all" && (0, ZQ.alwaysValidSchema)(G, Y)) return;
    let z = (0, PQ.allSchemaProperties)($.properties), K = (0, PQ.allSchemaProperties)($.patternProperties);
    U(), Q.ok(U1._`${W} === ${jI.default.errors}`);
    function U() {
      X.forIn("key", J, (M) => {
        if (!z.length && !K.length) L(M);
        else X.if(q(M), () => L(M));
      });
    }
    function q(M) {
      let O;
      if (z.length > 8) {
        let A = (0, ZQ.schemaRefOrVal)(G, $.properties, "properties");
        O = (0, PQ.isOwnProperty)(X, A, M);
      } else if (z.length) O = (0, U1.or)(...z.map((A) => U1._`${M} === ${A}`));
      else O = U1.nil;
      if (K.length) O = (0, U1.or)(O, ...K.map((A) => U1._`${(0, PQ.usePattern)(Q, A)}.test(${M})`));
      return (0, U1.not)(O);
    }
    function V(M) {
      X.code(U1._`delete ${J}[${M}]`);
    }
    function L(M) {
      if (B.removeAdditional === "all" || B.removeAdditional && Y === false) {
        V(M);
        return;
      }
      if (Y === false) {
        if (Q.setParams({ additionalProperty: M }), Q.error(), !H) X.break();
        return;
      }
      if (typeof Y == "object" && !(0, ZQ.alwaysValidSchema)(G, Y)) {
        let O = X.name("valid");
        if (B.removeAdditional === "failing") F(M, O, false), X.if((0, U1.not)(O), () => {
          Q.reset(), V(M);
        });
        else if (F(M, O), !H) X.if((0, U1.not)(O), () => X.break());
      }
    }
    function F(M, O, A) {
      let R = { keyword: "additionalProperties", dataProp: M, dataPropType: ZQ.Type.Str };
      if (A === false) Object.assign(R, { compositeRule: true, createErrors: false, allErrors: false });
      Q.subschema(R, O);
    }
  } };
  Tz.default = II;
});
var hz = E((gz) => {
  Object.defineProperty(gz, "__esModule", { value: true });
  var EI = E4(), xz = e0(), b7 = a(), yz = I7(), PI = { keyword: "properties", type: "object", schemaType: "object", code(Q) {
    let { gen: X, schema: Y, parentSchema: $, data: J, it: W } = Q;
    if (W.opts.removeAdditional === "all" && $.additionalProperties === void 0) yz.default.code(new EI.KeywordCxt(W, yz.default, "additionalProperties"));
    let G = (0, xz.allSchemaProperties)(Y);
    for (let U of G) W.definedProperties.add(U);
    if (W.opts.unevaluated && G.length && W.props !== true) W.props = b7.mergeEvaluated.props(X, (0, b7.toHash)(G), W.props);
    let H = G.filter((U) => !(0, b7.alwaysValidSchema)(W, Y[U]));
    if (H.length === 0) return;
    let B = X.name("valid");
    for (let U of H) {
      if (z(U)) K(U);
      else {
        if (X.if((0, xz.propertyInData)(X, J, U, W.opts.ownProperties)), K(U), !W.allErrors) X.else().var(B, true);
        X.endIf();
      }
      Q.it.definedProperties.add(U), Q.ok(B);
    }
    function z(U) {
      return W.opts.useDefaults && !W.compositeRule && Y[U].default !== void 0;
    }
    function K(U) {
      Q.subschema({ keyword: "properties", schemaProp: U, dataProp: U }, B);
    }
  } };
  gz.default = PI;
});
var cz = E((lz) => {
  Object.defineProperty(lz, "__esModule", { value: true });
  var fz = e0(), CQ = c(), uz = a(), mz = a(), CI = { keyword: "patternProperties", type: "object", schemaType: "object", code(Q) {
    let { gen: X, schema: Y, data: $, parentSchema: J, it: W } = Q, { opts: G } = W, H = (0, fz.allSchemaProperties)(Y), B = H.filter((F) => (0, uz.alwaysValidSchema)(W, Y[F]));
    if (H.length === 0 || B.length === H.length && (!W.opts.unevaluated || W.props === true)) return;
    let z = G.strictSchema && !G.allowMatchingProperties && J.properties, K = X.name("valid");
    if (W.props !== true && !(W.props instanceof CQ.Name)) W.props = (0, mz.evaluatedPropsToName)(X, W.props);
    let { props: U } = W;
    q();
    function q() {
      for (let F of H) {
        if (z) V(F);
        if (W.allErrors) L(F);
        else X.var(K, true), L(F), X.if(K);
      }
    }
    function V(F) {
      for (let M in z) if (new RegExp(F).test(M)) (0, uz.checkStrictMode)(W, `property ${M} matches pattern ${F} (use allowMatchingProperties)`);
    }
    function L(F) {
      X.forIn("key", $, (M) => {
        X.if(CQ._`${(0, fz.usePattern)(Q, F)}.test(${M})`, () => {
          let O = B.includes(F);
          if (!O) Q.subschema({ keyword: "patternProperties", schemaProp: F, dataProp: M, dataPropType: mz.Type.Str }, K);
          if (W.opts.unevaluated && U !== true) X.assign(CQ._`${U}[${M}]`, true);
          else if (!O && !W.allErrors) X.if((0, CQ.not)(K), () => X.break());
        });
      });
    }
  } };
  lz.default = CI;
});
var dz = E((pz) => {
  Object.defineProperty(pz, "__esModule", { value: true });
  var _I = a(), kI = { keyword: "not", schemaType: ["object", "boolean"], trackErrors: true, code(Q) {
    let { gen: X, schema: Y, it: $ } = Q;
    if ((0, _I.alwaysValidSchema)($, Y)) {
      Q.fail();
      return;
    }
    let J = X.name("valid");
    Q.subschema({ keyword: "not", compositeRule: true, createErrors: false, allErrors: false }, J), Q.failResult(J, () => Q.reset(), () => Q.error());
  }, error: { message: "must NOT be valid" } };
  pz.default = kI;
});
var nz = E((iz) => {
  Object.defineProperty(iz, "__esModule", { value: true });
  var TI = e0(), xI = { keyword: "anyOf", schemaType: "array", trackErrors: true, code: TI.validateUnion, error: { message: "must match a schema in anyOf" } };
  iz.default = xI;
});
var rz = E((oz) => {
  Object.defineProperty(oz, "__esModule", { value: true });
  var SQ = c(), gI = a(), hI = { message: "must match exactly one schema in oneOf", params: ({ params: Q }) => SQ._`{passingSchemas: ${Q.passing}}` }, fI = { keyword: "oneOf", schemaType: "array", trackErrors: true, error: hI, code(Q) {
    let { gen: X, schema: Y, parentSchema: $, it: J } = Q;
    if (!Array.isArray(Y)) throw Error("ajv implementation error");
    if (J.opts.discriminator && $.discriminator) return;
    let W = Y, G = X.let("valid", false), H = X.let("passing", null), B = X.name("_valid");
    Q.setParams({ passing: H }), X.block(z), Q.result(G, () => Q.reset(), () => Q.error(true));
    function z() {
      W.forEach((K, U) => {
        let q;
        if ((0, gI.alwaysValidSchema)(J, K)) X.var(B, true);
        else q = Q.subschema({ keyword: "oneOf", schemaProp: U, compositeRule: true }, B);
        if (U > 0) X.if(SQ._`${B} && ${G}`).assign(G, false).assign(H, SQ._`[${H}, ${U}]`).else();
        X.if(B, () => {
          if (X.assign(G, true), X.assign(H, U), q) Q.mergeEvaluated(q, SQ.Name);
        });
      });
    }
  } };
  oz.default = fI;
});
var az = E((tz) => {
  Object.defineProperty(tz, "__esModule", { value: true });
  var mI = a(), lI = { keyword: "allOf", schemaType: "array", code(Q) {
    let { gen: X, schema: Y, it: $ } = Q;
    if (!Array.isArray(Y)) throw Error("ajv implementation error");
    let J = X.name("valid");
    Y.forEach((W, G) => {
      if ((0, mI.alwaysValidSchema)($, W)) return;
      let H = Q.subschema({ keyword: "allOf", schemaProp: G }, J);
      Q.ok(J), Q.mergeEvaluated(H);
    });
  } };
  tz.default = lI;
});
var XK = E((QK) => {
  Object.defineProperty(QK, "__esModule", { value: true });
  var _Q = c(), ez = a(), pI = { message: ({ params: Q }) => _Q.str`must match "${Q.ifClause}" schema`, params: ({ params: Q }) => _Q._`{failingKeyword: ${Q.ifClause}}` }, dI = { keyword: "if", schemaType: ["object", "boolean"], trackErrors: true, error: pI, code(Q) {
    let { gen: X, parentSchema: Y, it: $ } = Q;
    if (Y.then === void 0 && Y.else === void 0) (0, ez.checkStrictMode)($, '"if" without "then" and "else" is ignored');
    let J = sz($, "then"), W = sz($, "else");
    if (!J && !W) return;
    let G = X.let("valid", true), H = X.name("_valid");
    if (B(), Q.reset(), J && W) {
      let K = X.let("ifClause");
      Q.setParams({ ifClause: K }), X.if(H, z("then", K), z("else", K));
    } else if (J) X.if(H, z("then"));
    else X.if((0, _Q.not)(H), z("else"));
    Q.pass(G, () => Q.error(true));
    function B() {
      let K = Q.subschema({ keyword: "if", compositeRule: true, createErrors: false, allErrors: false }, H);
      Q.mergeEvaluated(K);
    }
    function z(K, U) {
      return () => {
        let q = Q.subschema({ keyword: K }, H);
        if (X.assign(G, H), Q.mergeValidEvaluated(q, G), U) X.assign(U, _Q._`${K}`);
        else Q.setParams({ ifClause: K });
      };
    }
  } };
  function sz(Q, X) {
    let Y = Q.schema[X];
    return Y !== void 0 && !(0, ez.alwaysValidSchema)(Q, Y);
  }
  QK.default = dI;
});
var $K = E((YK) => {
  Object.defineProperty(YK, "__esModule", { value: true });
  var nI = a(), oI = { keyword: ["then", "else"], schemaType: ["object", "boolean"], code({ keyword: Q, parentSchema: X, it: Y }) {
    if (X.if === void 0) (0, nI.checkStrictMode)(Y, `"${Q}" without "if" is ignored`);
  } };
  YK.default = oI;
});
var WK = E((JK) => {
  Object.defineProperty(JK, "__esModule", { value: true });
  var tI = A7(), aI = Mz(), sI = j7(), eI = jz(), Qb = Iz(), Xb = Sz(), Yb = vz(), $b = I7(), Jb = hz(), Wb = cz(), Gb = dz(), Hb = nz(), Bb = rz(), zb = az(), Kb = XK(), Vb = $K();
  function qb(Q = false) {
    let X = [Gb.default, Hb.default, Bb.default, zb.default, Kb.default, Vb.default, Yb.default, $b.default, Xb.default, Jb.default, Wb.default];
    if (Q) X.push(aI.default, eI.default);
    else X.push(tI.default, sI.default);
    return X.push(Qb.default), X;
  }
  JK.default = qb;
});
var HK = E((GK) => {
  Object.defineProperty(GK, "__esModule", { value: true });
  var D0 = c(), Lb = { message: ({ schemaCode: Q }) => D0.str`must match format "${Q}"`, params: ({ schemaCode: Q }) => D0._`{format: ${Q}}` }, Fb = { keyword: "format", type: ["number", "string"], schemaType: "string", $data: true, error: Lb, code(Q, X) {
    let { gen: Y, data: $, $data: J, schema: W, schemaCode: G, it: H } = Q, { opts: B, errSchemaPath: z, schemaEnv: K, self: U } = H;
    if (!B.validateFormats) return;
    if (J) q();
    else V();
    function q() {
      let L = Y.scopeValue("formats", { ref: U.formats, code: B.code.formats }), F = Y.const("fDef", D0._`${L}[${G}]`), M = Y.let("fType"), O = Y.let("format");
      Y.if(D0._`typeof ${F} == "object" && !(${F} instanceof RegExp)`, () => Y.assign(M, D0._`${F}.type || "string"`).assign(O, D0._`${F}.validate`), () => Y.assign(M, D0._`"string"`).assign(O, F)), Q.fail$data((0, D0.or)(A(), R()));
      function A() {
        if (B.strictSchema === false) return D0.nil;
        return D0._`${G} && !${O}`;
      }
      function R() {
        let Z = K.$async ? D0._`(${F}.async ? await ${O}(${$}) : ${O}(${$}))` : D0._`${O}(${$})`, C = D0._`(typeof ${O} == "function" ? ${Z} : ${O}.test(${$}))`;
        return D0._`${O} && ${O} !== true && ${M} === ${X} && !${C}`;
      }
    }
    function V() {
      let L = U.formats[W];
      if (!L) {
        A();
        return;
      }
      if (L === true) return;
      let [F, M, O] = R(L);
      if (F === X) Q.pass(Z());
      function A() {
        if (B.strictSchema === false) {
          U.logger.warn(C());
          return;
        }
        throw Error(C());
        function C() {
          return `unknown format "${W}" ignored in schema at path "${z}"`;
        }
      }
      function R(C) {
        let B0 = C instanceof RegExp ? (0, D0.regexpCode)(C) : B.code.formats ? D0._`${B.code.formats}${(0, D0.getProperty)(W)}` : void 0, O0 = Y.scopeValue("formats", { key: W, ref: C, code: B0 });
        if (typeof C == "object" && !(C instanceof RegExp)) return [C.type || "string", C.validate, D0._`${O0}.validate`];
        return ["string", C, O0];
      }
      function Z() {
        if (typeof L == "object" && !(L instanceof RegExp) && L.async) {
          if (!K.$async) throw Error("async format in sync schema");
          return D0._`await ${O}(${$})`;
        }
        return typeof M == "function" ? D0._`${O}(${$})` : D0._`${O}.test(${$})`;
      }
    }
  } };
  GK.default = Fb;
});
var zK = E((BK) => {
  Object.defineProperty(BK, "__esModule", { value: true });
  var Ob = HK(), Db = [Ob.default];
  BK.default = Db;
});
var qK = E((KK) => {
  Object.defineProperty(KK, "__esModule", { value: true });
  KK.contentVocabulary = KK.metadataVocabulary = void 0;
  KK.metadataVocabulary = ["title", "description", "default", "deprecated", "readOnly", "writeOnly", "examples"];
  KK.contentVocabulary = ["contentMediaType", "contentEncoding", "contentSchema"];
});
var FK = E((LK) => {
  Object.defineProperty(LK, "__esModule", { value: true });
  var Ab = gB(), jb = Kz(), Rb = WK(), Ib = zK(), UK = qK(), bb = [Ab.default, jb.default, (0, Rb.default)(), Ib.default, UK.metadataVocabulary, UK.contentVocabulary];
  LK.default = bb;
});
var MK = E((OK) => {
  Object.defineProperty(OK, "__esModule", { value: true });
  OK.DiscrError = void 0;
  var NK;
  (function(Q) {
    Q.Tag = "tag", Q.Mapping = "mapping";
  })(NK || (OK.DiscrError = NK = {}));
});
var jK = E((AK) => {
  Object.defineProperty(AK, "__esModule", { value: true });
  var P9 = c(), E7 = MK(), wK = UQ(), Pb = P4(), Zb = a(), Cb = { message: ({ params: { discrError: Q, tagName: X } }) => Q === E7.DiscrError.Tag ? `tag "${X}" must be string` : `value of tag "${X}" must be in oneOf`, params: ({ params: { discrError: Q, tag: X, tagName: Y } }) => P9._`{error: ${Q}, tag: ${Y}, tagValue: ${X}}` }, Sb = { keyword: "discriminator", type: "object", schemaType: "object", error: Cb, code(Q) {
    let { gen: X, data: Y, schema: $, parentSchema: J, it: W } = Q, { oneOf: G } = J;
    if (!W.opts.discriminator) throw Error("discriminator: requires discriminator option");
    let H = $.propertyName;
    if (typeof H != "string") throw Error("discriminator: requires propertyName");
    if ($.mapping) throw Error("discriminator: mapping is not supported");
    if (!G) throw Error("discriminator: requires oneOf keyword");
    let B = X.let("valid", false), z = X.const("tag", P9._`${Y}${(0, P9.getProperty)(H)}`);
    X.if(P9._`typeof ${z} == "string"`, () => K(), () => Q.error(false, { discrError: E7.DiscrError.Tag, tag: z, tagName: H })), Q.ok(B);
    function K() {
      let V = q();
      X.if(false);
      for (let L in V) X.elseIf(P9._`${z} === ${L}`), X.assign(B, U(V[L]));
      X.else(), Q.error(false, { discrError: E7.DiscrError.Mapping, tag: z, tagName: H }), X.endIf();
    }
    function U(V) {
      let L = X.name("valid"), F = Q.subschema({ keyword: "oneOf", schemaProp: V }, L);
      return Q.mergeEvaluated(F, P9.Name), L;
    }
    function q() {
      var V;
      let L = {}, F = O(J), M = true;
      for (let Z = 0; Z < G.length; Z++) {
        let C = G[Z];
        if ((C === null || C === void 0 ? void 0 : C.$ref) && !(0, Zb.schemaHasRulesButRef)(C, W.self.RULES)) {
          let O0 = C.$ref;
          if (C = wK.resolveRef.call(W.self, W.schemaEnv.root, W.baseId, O0), C instanceof wK.SchemaEnv) C = C.schema;
          if (C === void 0) throw new Pb.default(W.opts.uriResolver, W.baseId, O0);
        }
        let B0 = (V = C === null || C === void 0 ? void 0 : C.properties) === null || V === void 0 ? void 0 : V[H];
        if (typeof B0 != "object") throw Error(`discriminator: oneOf subschemas (or referenced schemas) must have "properties/${H}"`);
        M = M && (F || O(C)), A(B0, Z);
      }
      if (!M) throw Error(`discriminator: "${H}" must be required`);
      return L;
      function O({ required: Z }) {
        return Array.isArray(Z) && Z.includes(H);
      }
      function A(Z, C) {
        if (Z.const) R(Z.const, C);
        else if (Z.enum) for (let B0 of Z.enum) R(B0, C);
        else throw Error(`discriminator: "properties/${H}" must have "const" or "enum"`);
      }
      function R(Z, C) {
        if (typeof Z != "string" || Z in L) throw Error(`discriminator: "${H}" values must be unique strings`);
        L[Z] = C;
      }
    }
  } };
  AK.default = Sb;
});
var RK = E((ry, kb) => {
  kb.exports = { $schema: "http://json-schema.org/draft-07/schema#", $id: "http://json-schema.org/draft-07/schema#", title: "Core schema meta-schema", definitions: { schemaArray: { type: "array", minItems: 1, items: { $ref: "#" } }, nonNegativeInteger: { type: "integer", minimum: 0 }, nonNegativeIntegerDefault0: { allOf: [{ $ref: "#/definitions/nonNegativeInteger" }, { default: 0 }] }, simpleTypes: { enum: ["array", "boolean", "integer", "null", "number", "object", "string"] }, stringArray: { type: "array", items: { type: "string" }, uniqueItems: true, default: [] } }, type: ["object", "boolean"], properties: { $id: { type: "string", format: "uri-reference" }, $schema: { type: "string", format: "uri" }, $ref: { type: "string", format: "uri-reference" }, $comment: { type: "string" }, title: { type: "string" }, description: { type: "string" }, default: true, readOnly: { type: "boolean", default: false }, examples: { type: "array", items: true }, multipleOf: { type: "number", exclusiveMinimum: 0 }, maximum: { type: "number" }, exclusiveMaximum: { type: "number" }, minimum: { type: "number" }, exclusiveMinimum: { type: "number" }, maxLength: { $ref: "#/definitions/nonNegativeInteger" }, minLength: { $ref: "#/definitions/nonNegativeIntegerDefault0" }, pattern: { type: "string", format: "regex" }, additionalItems: { $ref: "#" }, items: { anyOf: [{ $ref: "#" }, { $ref: "#/definitions/schemaArray" }], default: true }, maxItems: { $ref: "#/definitions/nonNegativeInteger" }, minItems: { $ref: "#/definitions/nonNegativeIntegerDefault0" }, uniqueItems: { type: "boolean", default: false }, contains: { $ref: "#" }, maxProperties: { $ref: "#/definitions/nonNegativeInteger" }, minProperties: { $ref: "#/definitions/nonNegativeIntegerDefault0" }, required: { $ref: "#/definitions/stringArray" }, additionalProperties: { $ref: "#" }, definitions: { type: "object", additionalProperties: { $ref: "#" }, default: {} }, properties: { type: "object", additionalProperties: { $ref: "#" }, default: {} }, patternProperties: { type: "object", additionalProperties: { $ref: "#" }, propertyNames: { format: "regex" }, default: {} }, dependencies: { type: "object", additionalProperties: { anyOf: [{ $ref: "#" }, { $ref: "#/definitions/stringArray" }] } }, propertyNames: { $ref: "#" }, const: true, enum: { type: "array", items: true, minItems: 1, uniqueItems: true }, type: { anyOf: [{ $ref: "#/definitions/simpleTypes" }, { type: "array", items: { $ref: "#/definitions/simpleTypes" }, minItems: 1, uniqueItems: true }] }, format: { type: "string" }, contentMediaType: { type: "string" }, contentEncoding: { type: "string" }, if: { $ref: "#" }, then: { $ref: "#" }, else: { $ref: "#" }, allOf: { $ref: "#/definitions/schemaArray" }, anyOf: { $ref: "#/definitions/schemaArray" }, oneOf: { $ref: "#/definitions/schemaArray" }, not: { $ref: "#" } }, default: true };
});
var Z7 = E((u0, P7) => {
  Object.defineProperty(u0, "__esModule", { value: true });
  u0.MissingRefError = u0.ValidationError = u0.CodeGen = u0.Name = u0.nil = u0.stringify = u0.str = u0._ = u0.KeywordCxt = u0.Ajv = void 0;
  var vb = PB(), Tb = FK(), xb = jK(), IK = RK(), yb = ["/properties"], kQ = "http://json-schema.org/draft-07/schema";
  class f4 extends vb.default {
    _addVocabularies() {
      if (super._addVocabularies(), Tb.default.forEach((Q) => this.addVocabulary(Q)), this.opts.discriminator) this.addKeyword(xb.default);
    }
    _addDefaultMetaSchema() {
      if (super._addDefaultMetaSchema(), !this.opts.meta) return;
      let Q = this.opts.$data ? this.$dataMetaSchema(IK, yb) : IK;
      this.addMetaSchema(Q, kQ, false), this.refs["http://json-schema.org/schema"] = kQ;
    }
    defaultMeta() {
      return this.opts.defaultMeta = super.defaultMeta() || (this.getSchema(kQ) ? kQ : void 0);
    }
  }
  u0.Ajv = f4;
  P7.exports = u0 = f4;
  P7.exports.Ajv = f4;
  Object.defineProperty(u0, "__esModule", { value: true });
  u0.default = f4;
  var gb = E4();
  Object.defineProperty(u0, "KeywordCxt", { enumerable: true, get: function() {
    return gb.KeywordCxt;
  } });
  var Z9 = c();
  Object.defineProperty(u0, "_", { enumerable: true, get: function() {
    return Z9._;
  } });
  Object.defineProperty(u0, "str", { enumerable: true, get: function() {
    return Z9.str;
  } });
  Object.defineProperty(u0, "stringify", { enumerable: true, get: function() {
    return Z9.stringify;
  } });
  Object.defineProperty(u0, "nil", { enumerable: true, get: function() {
    return Z9.nil;
  } });
  Object.defineProperty(u0, "Name", { enumerable: true, get: function() {
    return Z9.Name;
  } });
  Object.defineProperty(u0, "CodeGen", { enumerable: true, get: function() {
    return Z9.CodeGen;
  } });
  var hb = VQ();
  Object.defineProperty(u0, "ValidationError", { enumerable: true, get: function() {
    return hb.default;
  } });
  var fb = P4();
  Object.defineProperty(u0, "MissingRefError", { enumerable: true, get: function() {
    return fb.default;
  } });
});
var TK = E((kK) => {
  Object.defineProperty(kK, "__esModule", { value: true });
  kK.formatNames = kK.fastFormats = kK.fullFormats = void 0;
  function b1(Q, X) {
    return { validate: Q, compare: X };
  }
  kK.fullFormats = { date: b1(ZK, k7), time: b1(S7(true), v7), "date-time": b1(bK(true), SK), "iso-time": b1(S7(), CK), "iso-date-time": b1(bK(), _K), duration: /^P(?!$)((\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+S)?)?|(\d+W)?)$/, uri: nb, "uri-reference": /^(?:[a-z][a-z0-9+\-.]*:)?(?:\/?\/(?:(?:[a-z0-9\-._~!$&'()*+,;=:]|%[0-9a-f]{2})*@)?(?:\[(?:(?:(?:(?:[0-9a-f]{1,4}:){6}|::(?:[0-9a-f]{1,4}:){5}|(?:[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){4}|(?:(?:[0-9a-f]{1,4}:){0,1}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){3}|(?:(?:[0-9a-f]{1,4}:){0,2}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){2}|(?:(?:[0-9a-f]{1,4}:){0,3}[0-9a-f]{1,4})?::[0-9a-f]{1,4}:|(?:(?:[0-9a-f]{1,4}:){0,4}[0-9a-f]{1,4})?::)(?:[0-9a-f]{1,4}:[0-9a-f]{1,4}|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?))|(?:(?:[0-9a-f]{1,4}:){0,5}[0-9a-f]{1,4})?::[0-9a-f]{1,4}|(?:(?:[0-9a-f]{1,4}:){0,6}[0-9a-f]{1,4})?::)|[Vv][0-9a-f]+\.[a-z0-9\-._~!$&'()*+,;=:]+)\]|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)|(?:[a-z0-9\-._~!$&'"()*+,;=]|%[0-9a-f]{2})*)(?::\d*)?(?:\/(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})*)*|\/(?:(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})*)*)?|(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'"()*+,;=:@]|%[0-9a-f]{2})*)*)?(?:\?(?:[a-z0-9\-._~!$&'"()*+,;=:@/?]|%[0-9a-f]{2})*)?(?:#(?:[a-z0-9\-._~!$&'"()*+,;=:@/?]|%[0-9a-f]{2})*)?$/i, "uri-template": /^(?:(?:[^\x00-\x20"'<>%\\^`{|}]|%[0-9a-f]{2})|\{[+#./;?&=,!@|]?(?:[a-z0-9_]|%[0-9a-f]{2})+(?::[1-9][0-9]{0,3}|\*)?(?:,(?:[a-z0-9_]|%[0-9a-f]{2})+(?::[1-9][0-9]{0,3}|\*)?)*\})*$/i, url: /^(?:https?|ftp):\/\/(?:\S+(?::\S*)?@)?(?:(?!(?:10|127)(?:\.\d{1,3}){3})(?!(?:169\.254|192\.168)(?:\.\d{1,3}){2})(?!172\.(?:1[6-9]|2\d|3[0-1])(?:\.\d{1,3}){2})(?:[1-9]\d?|1\d\d|2[01]\d|22[0-3])(?:\.(?:1?\d{1,2}|2[0-4]\d|25[0-5])){2}(?:\.(?:[1-9]\d?|1\d\d|2[0-4]\d|25[0-4]))|(?:(?:[a-z0-9\u{00a1}-\u{ffff}]+-)*[a-z0-9\u{00a1}-\u{ffff}]+)(?:\.(?:[a-z0-9\u{00a1}-\u{ffff}]+-)*[a-z0-9\u{00a1}-\u{ffff}]+)*(?:\.(?:[a-z\u{00a1}-\u{ffff}]{2,})))(?::\d{2,5})?(?:\/[^\s]*)?$/iu, email: /^[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*@(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/i, hostname: /^(?=.{1,253}\.?$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[-0-9a-z]{0,61}[0-9a-z])?)*\.?$/i, ipv4: /^(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)\.){3}(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)$/, ipv6: /^((([0-9a-f]{1,4}:){7}([0-9a-f]{1,4}|:))|(([0-9a-f]{1,4}:){6}(:[0-9a-f]{1,4}|((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){5}(((:[0-9a-f]{1,4}){1,2})|:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3})|:))|(([0-9a-f]{1,4}:){4}(((:[0-9a-f]{1,4}){1,3})|((:[0-9a-f]{1,4})?:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){3}(((:[0-9a-f]{1,4}){1,4})|((:[0-9a-f]{1,4}){0,2}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){2}(((:[0-9a-f]{1,4}){1,5})|((:[0-9a-f]{1,4}){0,3}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(([0-9a-f]{1,4}:){1}(((:[0-9a-f]{1,4}){1,6})|((:[0-9a-f]{1,4}){0,4}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:))|(:(((:[0-9a-f]{1,4}){1,7})|((:[0-9a-f]{1,4}){0,5}:((25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}))|:)))$/i, regex: QE, uuid: /^(?:urn:uuid:)?[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}$/i, "json-pointer": /^(?:\/(?:[^~/]|~0|~1)*)*$/, "json-pointer-uri-fragment": /^#(?:\/(?:[a-z0-9_\-.!$&'()*+,;:=@]|%[0-9a-f]{2}|~0|~1)*)*$/i, "relative-json-pointer": /^(?:0|[1-9][0-9]*)(?:#|(?:\/(?:[^~/]|~0|~1)*)*)$/, byte: ob, int32: { type: "number", validate: ab }, int64: { type: "number", validate: sb }, float: { type: "number", validate: PK }, double: { type: "number", validate: PK }, password: true, binary: true };
  kK.fastFormats = { ...kK.fullFormats, date: b1(/^\d\d\d\d-[0-1]\d-[0-3]\d$/, k7), time: b1(/^(?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)$/i, v7), "date-time": b1(/^\d\d\d\d-[0-1]\d-[0-3]\dt(?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)$/i, SK), "iso-time": b1(/^(?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)?$/i, CK), "iso-date-time": b1(/^\d\d\d\d-[0-1]\d-[0-3]\d[t\s](?:[0-2]\d:[0-5]\d:[0-5]\d|23:59:60)(?:\.\d+)?(?:z|[+-]\d\d(?::?\d\d)?)?$/i, _K), uri: /^(?:[a-z][a-z0-9+\-.]*:)(?:\/?\/)?[^\s]*$/i, "uri-reference": /^(?:(?:[a-z][a-z0-9+\-.]*:)?\/?\/)?(?:[^\\\s#][^\s#]*)?(?:#[^\\\s]*)?$/i, email: /^[a-z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$/i };
  kK.formatNames = Object.keys(kK.fullFormats);
  function lb(Q) {
    return Q % 4 === 0 && (Q % 100 !== 0 || Q % 400 === 0);
  }
  var cb = /^(\d\d\d\d)-(\d\d)-(\d\d)$/, pb = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  function ZK(Q) {
    let X = cb.exec(Q);
    if (!X) return false;
    let Y = +X[1], $ = +X[2], J = +X[3];
    return $ >= 1 && $ <= 12 && J >= 1 && J <= ($ === 2 && lb(Y) ? 29 : pb[$]);
  }
  function k7(Q, X) {
    if (!(Q && X)) return;
    if (Q > X) return 1;
    if (Q < X) return -1;
    return 0;
  }
  var C7 = /^(\d\d):(\d\d):(\d\d(?:\.\d+)?)(z|([+-])(\d\d)(?::?(\d\d))?)?$/i;
  function S7(Q) {
    return function(Y) {
      let $ = C7.exec(Y);
      if (!$) return false;
      let J = +$[1], W = +$[2], G = +$[3], H = $[4], B = $[5] === "-" ? -1 : 1, z = +($[6] || 0), K = +($[7] || 0);
      if (z > 23 || K > 59 || Q && !H) return false;
      if (J <= 23 && W <= 59 && G < 60) return true;
      let U = W - K * B, q = J - z * B - (U < 0 ? 1 : 0);
      return (q === 23 || q === -1) && (U === 59 || U === -1) && G < 61;
    };
  }
  function v7(Q, X) {
    if (!(Q && X)) return;
    let Y = (/* @__PURE__ */ new Date("2020-01-01T" + Q)).valueOf(), $ = (/* @__PURE__ */ new Date("2020-01-01T" + X)).valueOf();
    if (!(Y && $)) return;
    return Y - $;
  }
  function CK(Q, X) {
    if (!(Q && X)) return;
    let Y = C7.exec(Q), $ = C7.exec(X);
    if (!(Y && $)) return;
    if (Q = Y[1] + Y[2] + Y[3], X = $[1] + $[2] + $[3], Q > X) return 1;
    if (Q < X) return -1;
    return 0;
  }
  var _7 = /t|\s/i;
  function bK(Q) {
    let X = S7(Q);
    return function($) {
      let J = $.split(_7);
      return J.length === 2 && ZK(J[0]) && X(J[1]);
    };
  }
  function SK(Q, X) {
    if (!(Q && X)) return;
    let Y = new Date(Q).valueOf(), $ = new Date(X).valueOf();
    if (!(Y && $)) return;
    return Y - $;
  }
  function _K(Q, X) {
    if (!(Q && X)) return;
    let [Y, $] = Q.split(_7), [J, W] = X.split(_7), G = k7(Y, J);
    if (G === void 0) return;
    return G || v7($, W);
  }
  var db = /\/|:/, ib = /^(?:[a-z][a-z0-9+\-.]*:)(?:\/?\/(?:(?:[a-z0-9\-._~!$&'()*+,;=:]|%[0-9a-f]{2})*@)?(?:\[(?:(?:(?:(?:[0-9a-f]{1,4}:){6}|::(?:[0-9a-f]{1,4}:){5}|(?:[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){4}|(?:(?:[0-9a-f]{1,4}:){0,1}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){3}|(?:(?:[0-9a-f]{1,4}:){0,2}[0-9a-f]{1,4})?::(?:[0-9a-f]{1,4}:){2}|(?:(?:[0-9a-f]{1,4}:){0,3}[0-9a-f]{1,4})?::[0-9a-f]{1,4}:|(?:(?:[0-9a-f]{1,4}:){0,4}[0-9a-f]{1,4})?::)(?:[0-9a-f]{1,4}:[0-9a-f]{1,4}|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?))|(?:(?:[0-9a-f]{1,4}:){0,5}[0-9a-f]{1,4})?::[0-9a-f]{1,4}|(?:(?:[0-9a-f]{1,4}:){0,6}[0-9a-f]{1,4})?::)|[Vv][0-9a-f]+\.[a-z0-9\-._~!$&'()*+,;=:]+)\]|(?:(?:25[0-5]|2[0-4]\d|[01]?\d\d?)\.){3}(?:25[0-5]|2[0-4]\d|[01]?\d\d?)|(?:[a-z0-9\-._~!$&'()*+,;=]|%[0-9a-f]{2})*)(?::\d*)?(?:\/(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})*)*|\/(?:(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})*)*)?|(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})+(?:\/(?:[a-z0-9\-._~!$&'()*+,;=:@]|%[0-9a-f]{2})*)*)(?:\?(?:[a-z0-9\-._~!$&'()*+,;=:@/?]|%[0-9a-f]{2})*)?(?:#(?:[a-z0-9\-._~!$&'()*+,;=:@/?]|%[0-9a-f]{2})*)?$/i;
  function nb(Q) {
    return db.test(Q) && ib.test(Q);
  }
  var EK = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/gm;
  function ob(Q) {
    return EK.lastIndex = 0, EK.test(Q);
  }
  var rb = -2147483648, tb = 2147483647;
  function ab(Q) {
    return Number.isInteger(Q) && Q <= tb && Q >= rb;
  }
  function sb(Q) {
    return Number.isInteger(Q);
  }
  function PK() {
    return true;
  }
  var eb = /[^\\]\\Z/;
  function QE(Q) {
    if (eb.test(Q)) return false;
    try {
      return new RegExp(Q), true;
    } catch (X) {
      return false;
    }
  }
});
var yK = E((xK) => {
  Object.defineProperty(xK, "__esModule", { value: true });
  xK.formatLimitDefinition = void 0;
  var YE = Z7(), L1 = c(), G6 = L1.operators, vQ = { formatMaximum: { okStr: "<=", ok: G6.LTE, fail: G6.GT }, formatMinimum: { okStr: ">=", ok: G6.GTE, fail: G6.LT }, formatExclusiveMaximum: { okStr: "<", ok: G6.LT, fail: G6.GTE }, formatExclusiveMinimum: { okStr: ">", ok: G6.GT, fail: G6.LTE } }, $E = { message: ({ keyword: Q, schemaCode: X }) => L1.str`should be ${vQ[Q].okStr} ${X}`, params: ({ keyword: Q, schemaCode: X }) => L1._`{comparison: ${vQ[Q].okStr}, limit: ${X}}` };
  xK.formatLimitDefinition = { keyword: Object.keys(vQ), type: "string", schemaType: "string", $data: true, error: $E, code(Q) {
    let { gen: X, data: Y, schemaCode: $, keyword: J, it: W } = Q, { opts: G, self: H } = W;
    if (!G.validateFormats) return;
    let B = new YE.KeywordCxt(W, H.RULES.all.format.definition, "format");
    if (B.$data) z();
    else K();
    function z() {
      let q = X.scopeValue("formats", { ref: H.formats, code: G.code.formats }), V = X.const("fmt", L1._`${q}[${B.schemaCode}]`);
      Q.fail$data((0, L1.or)(L1._`typeof ${V} != "object"`, L1._`${V} instanceof RegExp`, L1._`typeof ${V}.compare != "function"`, U(V)));
    }
    function K() {
      let q = B.schema, V = H.formats[q];
      if (!V || V === true) return;
      if (typeof V != "object" || V instanceof RegExp || typeof V.compare != "function") throw Error(`"${J}": format "${q}" does not define "compare" function`);
      let L = X.scopeValue("formats", { key: q, ref: V, code: G.code.formats ? L1._`${G.code.formats}${(0, L1.getProperty)(q)}` : void 0 });
      Q.fail$data(U(L));
    }
    function U(q) {
      return L1._`${q}.compare(${Y}, ${$}) ${vQ[J].fail} 0`;
    }
  }, dependencies: ["format"] };
  var JE = (Q) => {
    return Q.addKeyword(xK.formatLimitDefinition), Q;
  };
  xK.default = JE;
});
var uK = E((u4, fK) => {
  Object.defineProperty(u4, "__esModule", { value: true });
  var C9 = TK(), GE = yK(), y7 = c(), gK = new y7.Name("fullFormats"), HE = new y7.Name("fastFormats"), g7 = (Q, X = { keywords: true }) => {
    if (Array.isArray(X)) return hK(Q, X, C9.fullFormats, gK), Q;
    let [Y, $] = X.mode === "fast" ? [C9.fastFormats, HE] : [C9.fullFormats, gK], J = X.formats || C9.formatNames;
    if (hK(Q, J, Y, $), X.keywords) (0, GE.default)(Q);
    return Q;
  };
  g7.get = (Q, X = "full") => {
    let $ = (X === "fast" ? C9.fastFormats : C9.fullFormats)[Q];
    if (!$) throw Error(`Unknown format "${Q}"`);
    return $;
  };
  function hK(Q, X, Y, $) {
    var J, W;
    (J = (W = Q.opts.code).formats) !== null && J !== void 0 || (W.formats = y7._`require("ajv-formats/dist/formats").${$}`);
    for (let G of X) Q.addFormat(G, Y[G]);
  }
  fK.exports = u4 = g7;
  Object.defineProperty(u4, "__esModule", { value: true });
  u4.default = g7;
});
var EV = 50;
function y6(Q = EV) {
  let X = new AbortController();
  return bV(Q, X.signal), X;
}
var T0 = class extends Error {
};
function g6() {
  return process.versions.bun !== void 0;
}
var CV = typeof global == "object" && global && global.Object === Object && global;
var e7 = CV;
var SV = typeof self == "object" && self && self.Object === Object && self;
var _V = e7 || SV || Function("return this")();
var h6 = _V;
var kV = h6.Symbol;
var f6 = kV;
var Q5 = Object.prototype;
var vV = Q5.hasOwnProperty;
var TV = Q5.toString;
var T9 = f6 ? f6.toStringTag : void 0;
function xV(Q) {
  var X = vV.call(Q, T9), Y = Q[T9];
  try {
    Q[T9] = void 0;
    var $ = true;
  } catch (W) {
  }
  var J = TV.call(Q);
  if ($) if (X) Q[T9] = Y;
  else delete Q[T9];
  return J;
}
var X5 = xV;
var yV = Object.prototype;
var gV = yV.toString;
function hV(Q) {
  return gV.call(Q);
}
var Y5 = hV;
var fV = "[object Null]";
var uV = "[object Undefined]";
var $5 = f6 ? f6.toStringTag : void 0;
function mV(Q) {
  if (Q == null) return Q === void 0 ? uV : fV;
  return $5 && $5 in Object(Q) ? X5(Q) : Y5(Q);
}
var J5 = mV;
function lV(Q) {
  var X = typeof Q;
  return Q != null && (X == "object" || X == "function");
}
var c4 = lV;
var cV = "[object AsyncFunction]";
var pV = "[object Function]";
var dV = "[object GeneratorFunction]";
var iV = "[object Proxy]";
function nV(Q) {
  if (!c4(Q)) return false;
  var X = J5(Q);
  return X == pV || X == dV || X == cV || X == iV;
}
var W5 = nV;
var oV = h6["__core-js_shared__"];
var p4 = oV;
var G5 = (function() {
  var Q = /[^.]+$/.exec(p4 && p4.keys && p4.keys.IE_PROTO || "");
  return Q ? "Symbol(src)_1." + Q : "";
})();
function rV(Q) {
  return !!G5 && G5 in Q;
}
var H5 = rV;
var tV = Function.prototype;
var aV = tV.toString;
function sV(Q) {
  if (Q != null) {
    try {
      return aV.call(Q);
    } catch (X) {
    }
    try {
      return Q + "";
    } catch (X) {
    }
  }
  return "";
}
var B5 = sV;
var eV = /[\\^$.*+?()[\]{}|]/g;
var Qq = /^\[object .+?Constructor\]$/;
var Xq = Function.prototype;
var Yq = Object.prototype;
var $q = Xq.toString;
var Jq = Yq.hasOwnProperty;
var Wq = RegExp("^" + $q.call(Jq).replace(eV, "\\$&").replace(/hasOwnProperty|(function).*?(?=\\\()| for .+?(?=\\\])/g, "$1.*?") + "$");
function Gq(Q) {
  if (!c4(Q) || H5(Q)) return false;
  var X = W5(Q) ? Wq : Qq;
  return X.test(B5(Q));
}
var z5 = Gq;
function Hq(Q, X) {
  return Q == null ? void 0 : Q[X];
}
var K5 = Hq;
function Bq(Q, X) {
  var Y = K5(Q, X);
  return z5(Y) ? Y : void 0;
}
var d4 = Bq;
var zq = d4(Object, "create");
var P1 = zq;
function Kq() {
  this.__data__ = P1 ? P1(null) : {}, this.size = 0;
}
var V5 = Kq;
function Vq(Q) {
  var X = this.has(Q) && delete this.__data__[Q];
  return this.size -= X ? 1 : 0, X;
}
var q5 = Vq;
var qq = "__lodash_hash_undefined__";
var Uq = Object.prototype;
var Lq = Uq.hasOwnProperty;
function Fq(Q) {
  var X = this.__data__;
  if (P1) {
    var Y = X[Q];
    return Y === qq ? void 0 : Y;
  }
  return Lq.call(X, Q) ? X[Q] : void 0;
}
var U5 = Fq;
var Nq = Object.prototype;
var Oq = Nq.hasOwnProperty;
function Dq(Q) {
  var X = this.__data__;
  return P1 ? X[Q] !== void 0 : Oq.call(X, Q);
}
var L5 = Dq;
var Mq = "__lodash_hash_undefined__";
function wq(Q, X) {
  var Y = this.__data__;
  return this.size += this.has(Q) ? 0 : 1, Y[Q] = P1 && X === void 0 ? Mq : X, this;
}
var F5 = wq;
function u6(Q) {
  var X = -1, Y = Q == null ? 0 : Q.length;
  this.clear();
  while (++X < Y) {
    var $ = Q[X];
    this.set($[0], $[1]);
  }
}
u6.prototype.clear = V5;
u6.prototype.delete = q5;
u6.prototype.get = U5;
u6.prototype.has = L5;
u6.prototype.set = F5;
var hQ = u6;
function Aq() {
  this.__data__ = [], this.size = 0;
}
var N5 = Aq;
function jq(Q, X) {
  return Q === X || Q !== Q && X !== X;
}
var O5 = jq;
function Rq(Q, X) {
  var Y = Q.length;
  while (Y--) if (O5(Q[Y][0], X)) return Y;
  return -1;
}
var h1 = Rq;
var Iq = Array.prototype;
var bq = Iq.splice;
function Eq(Q) {
  var X = this.__data__, Y = h1(X, Q);
  if (Y < 0) return false;
  var $ = X.length - 1;
  if (Y == $) X.pop();
  else bq.call(X, Y, 1);
  return --this.size, true;
}
var D5 = Eq;
function Pq(Q) {
  var X = this.__data__, Y = h1(X, Q);
  return Y < 0 ? void 0 : X[Y][1];
}
var M5 = Pq;
function Zq(Q) {
  return h1(this.__data__, Q) > -1;
}
var w5 = Zq;
function Cq(Q, X) {
  var Y = this.__data__, $ = h1(Y, Q);
  if ($ < 0) ++this.size, Y.push([Q, X]);
  else Y[$][1] = X;
  return this;
}
var A5 = Cq;
function m6(Q) {
  var X = -1, Y = Q == null ? 0 : Q.length;
  this.clear();
  while (++X < Y) {
    var $ = Q[X];
    this.set($[0], $[1]);
  }
}
m6.prototype.clear = N5;
m6.prototype.delete = D5;
m6.prototype.get = M5;
m6.prototype.has = w5;
m6.prototype.set = A5;
var j5 = m6;
var Sq = d4(h6, "Map");
var R5 = Sq;
function _q() {
  this.size = 0, this.__data__ = { hash: new hQ(), map: new (R5 || j5)(), string: new hQ() };
}
var I5 = _q;
function kq(Q) {
  var X = typeof Q;
  return X == "string" || X == "number" || X == "symbol" || X == "boolean" ? Q !== "__proto__" : Q === null;
}
var b5 = kq;
function vq(Q, X) {
  var Y = Q.__data__;
  return b5(X) ? Y[typeof X == "string" ? "string" : "hash"] : Y.map;
}
var f1 = vq;
function Tq(Q) {
  var X = f1(this, Q).delete(Q);
  return this.size -= X ? 1 : 0, X;
}
var E5 = Tq;
function xq(Q) {
  return f1(this, Q).get(Q);
}
var P5 = xq;
function yq(Q) {
  return f1(this, Q).has(Q);
}
var Z5 = yq;
function gq(Q, X) {
  var Y = f1(this, Q), $ = Y.size;
  return Y.set(Q, X), this.size += Y.size == $ ? 0 : 1, this;
}
var C5 = gq;
function l6(Q) {
  var X = -1, Y = Q == null ? 0 : Q.length;
  this.clear();
  while (++X < Y) {
    var $ = Q[X];
    this.set($[0], $[1]);
  }
}
l6.prototype.clear = I5;
l6.prototype.delete = E5;
l6.prototype.get = P5;
l6.prototype.has = Z5;
l6.prototype.set = C5;
var fQ = l6;
var hq = "Expected a function";
function uQ(Q, X) {
  if (typeof Q != "function" || X != null && typeof X != "function") throw TypeError(hq);
  var Y = function() {
    var $ = arguments, J = X ? X.apply(this, $) : $[0], W = Y.cache;
    if (W.has(J)) return W.get(J);
    var G = Q.apply(this, $);
    return Y.cache = W.set(J, G) || W, G;
  };
  return Y.cache = new (uQ.Cache || fQ)(), Y;
}
uQ.Cache = fQ;
var X1 = uQ;
var c6 = X1(() => {
  return (process.env.CLAUDE_CONFIG_DIR ?? fq(uq(), ".claude")).normalize("NFC");
}, () => process.env.CLAUDE_CONFIG_DIR);
function x9(Q) {
  if (!Q) return false;
  if (typeof Q === "boolean") return Q;
  let X = Q.toLowerCase().trim();
  return ["1", "true", "yes", "on"].includes(X);
}
var d6;
var p6 = null;
function pq() {
  if (p6) return p6;
  if (!process.env.DEBUG_CLAUDE_AGENT_SDK) return d6 = null, p6 = Promise.resolve(), p6;
  let Q = S5(c6(), "debug");
  return d6 = S5(Q, `sdk-${mq()}.txt`), process.stderr.write(`SDK debug logs: ${d6}
`), p6 = cq(Q, { recursive: true }).then(() => {
  }).catch(() => {
  }), p6;
}
function i0(Q) {
  if (d6 === null) return;
  let Y = `${(/* @__PURE__ */ new Date()).toISOString()} ${Q}
`;
  pq().then(() => {
    if (d6) lq(d6, Y).catch(() => {
    });
  });
}
function dq(Q, X) {
  if (Q.destroyed) return;
  Q.write(X);
}
function _5(Q) {
  dq(process.stderr, Q);
}
var k5 = X1((Q) => {
  if (!Q || Q.trim() === "") return null;
  let X = Q.split(",").map((W) => W.trim()).filter(Boolean);
  if (X.length === 0) return null;
  let Y = X.some((W) => W.startsWith("!")), $ = X.some((W) => !W.startsWith("!"));
  if (Y && $) return null;
  let J = X.map((W) => W.replace(/^!/, "").toLowerCase());
  return { include: Y ? [] : J, exclude: Y ? J : [], isExclusive: Y };
});
function iq(Q) {
  let X = [], Y = Q.match(/^MCP server ["']([^"']+)["']/);
  if (Y && Y[1]) X.push("mcp"), X.push(Y[1].toLowerCase());
  else {
    let W = Q.match(/^([^:[]+):/);
    if (W && W[1]) X.push(W[1].trim().toLowerCase());
  }
  let $ = Q.match(/^\[([^\]]+)]/);
  if ($ && $[1]) X.push($[1].trim().toLowerCase());
  if (Q.toLowerCase().includes("1p event:")) X.push("1p");
  let J = Q.match(/:\s*([^:]+?)(?:\s+(?:type|mode|status|event))?:/);
  if (J && J[1]) {
    let W = J[1].trim().toLowerCase();
    if (W.length < 30 && !W.includes(" ")) X.push(W);
  }
  return Array.from(new Set(X));
}
function nq(Q, X) {
  if (!X) return true;
  if (Q.length === 0) return false;
  if (X.isExclusive) return !Q.some((Y) => X.exclude.includes(Y));
  else return Q.some((Y) => X.include.includes(Y));
}
function v5(Q, X) {
  if (!X) return true;
  let Y = iq(Q);
  return nq(Y, X);
}
var YU = { cwd() {
  return process.cwd();
}, existsSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.existsSync(${Q})`, 0);
    return u.existsSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, async stat(Q) {
  return oq(Q);
}, async readdir(Q) {
  return rq(Q, { withFileTypes: true });
}, async unlink(Q) {
  return tq(Q);
}, async rmdir(Q) {
  return aq(Q);
}, async rm(Q, X) {
  return sq(Q, X);
}, async mkdir(Q, X) {
  try {
    await eq(Q, { recursive: true, ...X });
  } catch (Y) {
    if (Y.code !== "EEXIST") throw Y;
  }
}, async readFile(Q, X) {
  return T5(Q, { encoding: X.encoding });
}, async rename(Q, X) {
  return QU(Q, X);
}, statSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.statSync(${Q})`, 0);
    return u.statSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, lstatSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.lstatSync(${Q})`, 0);
    return u.lstatSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, readFileSync(Q, X) {
  let $ = [];
  try {
    const Y = X0($, U0`fs.readFileSync(${Q})`, 0);
    return u.readFileSync(Q, { encoding: X.encoding });
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
}, readFileBytesSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.readFileBytesSync(${Q})`, 0);
    return u.readFileSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, readSync(Q, X) {
  let J = [];
  try {
    const Y = X0(J, U0`fs.readSync(${Q}, ${X.length} bytes)`, 0);
    let $ = void 0;
    try {
      $ = u.openSync(Q, "r");
      let B = Buffer.alloc(X.length), z = u.readSync($, B, 0, X.length, 0);
      return { buffer: B, bytesRead: z };
    } finally {
      if ($) u.closeSync($);
    }
  } catch (W) {
    var G = W, H = 1;
  } finally {
    Y0(J, G, H);
  }
}, appendFileSync(Q, X, Y) {
  let J = [];
  try {
    const $ = X0(J, U0`fs.appendFileSync(${Q}, ${X.length} chars)`, 0);
    if (Y?.mode !== void 0) try {
      let B = u.openSync(Q, "ax", Y.mode);
      try {
        u.appendFileSync(B, X);
      } finally {
        u.closeSync(B);
      }
      return;
    } catch (B) {
      if (B.code !== "EEXIST") throw B;
    }
    u.appendFileSync(Q, X);
  } catch (W) {
    var G = W, H = 1;
  } finally {
    Y0(J, G, H);
  }
}, copyFileSync(Q, X) {
  let $ = [];
  try {
    const Y = X0($, U0`fs.copyFileSync(${Q} → ${X})`, 0);
    u.copyFileSync(Q, X);
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
}, unlinkSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.unlinkSync(${Q})`, 0);
    u.unlinkSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, renameSync(Q, X) {
  let $ = [];
  try {
    const Y = X0($, U0`fs.renameSync(${Q} → ${X})`, 0);
    u.renameSync(Q, X);
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
}, linkSync(Q, X) {
  let $ = [];
  try {
    const Y = X0($, U0`fs.linkSync(${Q} → ${X})`, 0);
    u.linkSync(Q, X);
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
}, symlinkSync(Q, X, Y) {
  let J = [];
  try {
    const $ = X0(J, U0`fs.symlinkSync(${Q} → ${X})`, 0);
    u.symlinkSync(Q, X, Y);
  } catch (W) {
    var G = W, H = 1;
  } finally {
    Y0(J, G, H);
  }
}, readlinkSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.readlinkSync(${Q})`, 0);
    return u.readlinkSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, realpathSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.realpathSync(${Q})`, 0);
    return u.realpathSync(Q).normalize("NFC");
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, mkdirSync(Q, X) {
  let J = [];
  try {
    const Y = X0(J, U0`fs.mkdirSync(${Q})`, 0);
    let $ = { recursive: true };
    if (X?.mode !== void 0) $.mode = X.mode;
    try {
      u.mkdirSync(Q, $);
    } catch (B) {
      if (B.code !== "EEXIST") throw B;
    }
  } catch (W) {
    var G = W, H = 1;
  } finally {
    Y0(J, G, H);
  }
}, readdirSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.readdirSync(${Q})`, 0);
    return u.readdirSync(Q, { withFileTypes: true });
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, readdirStringSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.readdirStringSync(${Q})`, 0);
    return u.readdirSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, isDirEmptySync(Q) {
  let $ = [];
  try {
    const X = X0($, U0`fs.isDirEmptySync(${Q})`, 0);
    let Y = this.readdirSync(Q);
    return Y.length === 0;
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
}, rmdirSync(Q) {
  let Y = [];
  try {
    const X = X0(Y, U0`fs.rmdirSync(${Q})`, 0);
    u.rmdirSync(Q);
  } catch ($) {
    var J = $, W = 1;
  } finally {
    Y0(Y, J, W);
  }
}, rmSync(Q, X) {
  let $ = [];
  try {
    const Y = X0($, U0`fs.rmSync(${Q})`, 0);
    u.rmSync(Q, X);
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
}, createWriteStream(Q) {
  return u.createWriteStream(Q);
}, async readFileBytes(Q, X) {
  if (X === void 0) return T5(Q);
  let Y = await XU(Q, "r");
  try {
    let { size: $ } = await Y.stat(), J = Math.min($, X), W = Buffer.allocUnsafe(J), G = 0;
    while (G < J) {
      let { bytesRead: H } = await Y.read(W, G, J - G, G);
      if (H === 0) break;
      G += H;
    }
    return G < J ? W.subarray(0, G) : W;
  } finally {
    await Y.close();
  }
} };
var $U = YU;
function i6() {
  return $U;
}
function WU() {
  let Q = "";
  if (typeof process < "u" && typeof process.cwd === "function" && typeof x5 === "function") Q = x5(JU()).normalize("NFC");
  return { originalCwd: Q, projectRoot: Q, totalCostUSD: 0, totalAPIDuration: 0, totalAPIDurationWithoutRetries: 0, totalToolDuration: 0, tokenSaverBytesSaved: 0, tokenSaverHits: 0, turnHookDurationMs: 0, turnToolDurationMs: 0, turnClassifierDurationMs: 0, turnToolCount: 0, turnHookCount: 0, turnClassifierCount: 0, startTime: Date.now(), lastInteractionTime: Date.now(), totalLinesAdded: 0, totalLinesRemoved: 0, hasUnknownModelCost: false, cwd: Q, modelUsage: {}, mainLoopModelOverride: void 0, initialMainLoopModel: null, modelStrings: null, isInteractive: false, kairosActive: false, sdkAgentProgressSummariesEnabled: false, userMsgOptIn: false, clientType: "cli", sessionSource: void 0, questionPreviewFormat: void 0, sessionIngressToken: void 0, oauthTokenFromFd: void 0, apiKeyFromFd: void 0, flagSettingsPath: void 0, flagSettingsInline: null, allowedSettingSources: ["userSettings", "projectSettings", "localSettings", "flagSettings", "policySettings"], meter: null, sessionCounter: null, locCounter: null, prCounter: null, commitCounter: null, costCounter: null, tokenCounter: null, codeEditToolDecisionCounter: null, activeTimeCounter: null, statsStore: null, sessionId: i4(), parentSessionId: void 0, loggerProvider: null, eventLogger: null, meterProvider: null, tracerProvider: null, agentColorMap: /* @__PURE__ */ new Map(), agentColorIndex: 0, lastAPIRequest: null, lastClassifierRequests: null, inMemoryErrorLog: [], inlinePlugins: [], chromeFlagOverride: void 0, useCoworkPlugins: false, sessionBypassPermissionsMode: false, scheduledTasksEnabled: false, sessionCronTasks: [], sessionCreatedTeams: /* @__PURE__ */ new Set(), sessionTrustAccepted: false, sessionPersistenceDisabled: false, hasExitedPlanMode: false, needsPlanModeExitAttachment: false, needsAutoModeExitAttachment: false, lspRecommendationShownThisSession: false, initJsonSchema: null, registeredHooks: null, planSlugCache: /* @__PURE__ */ new Map(), teleportedSessionInfo: null, invokedSkills: /* @__PURE__ */ new Map(), slowOperations: [], sdkBetas: void 0, mainThreadAgentType: void 0, isRemoteMode: false, isInWorktree: false, ...{}, directConnectServerUrl: void 0, systemPromptSectionCache: /* @__PURE__ */ new Map(), lastEmittedDate: null, additionalDirectoriesForClaudeMd: [], allowedChannels: [], sessionProjectDir: null, promptCache1hAllowlist: null, promptId: null };
}
var GU = WU();
function y5() {
  return GU.sessionId;
}
function g5({ writeFn: Q, flushIntervalMs: X = 1e3, maxBufferSize: Y = 100, maxBufferBytes: $ = 1 / 0, immediateMode: J = false }) {
  let W = [], G = 0, H = null, B = null;
  function z() {
    if (H) clearTimeout(H), H = null;
  }
  function K() {
    if (B) Q(B.join("")), B = null;
    if (W.length === 0) return;
    Q(W.join("")), W = [], G = 0, z();
  }
  function U() {
    if (!H) H = setTimeout(K, X);
  }
  function q() {
    if (B) {
      B.push(...W), W = [], G = 0, z();
      return;
    }
    let V = W;
    W = [], G = 0, z(), B = V, setImmediate(() => {
      let L = B;
      if (B = null, L) Q(L.join(""));
    });
  }
  return { write(V) {
    if (J) {
      Q(V);
      return;
    }
    if (W.push(V), G += V.length, U(), W.length >= Y || G >= $) q();
  }, flush: K, dispose() {
    K();
  } };
}
var h5 = /* @__PURE__ */ new Set();
function f5(Q) {
  return h5.add(Q), () => h5.delete(Q);
}
var mQ = { verbose: 0, debug: 1, info: 2, warn: 3, error: 4 };
var HU = X1(() => {
  let Q = process.env.CLAUDE_CODE_DEBUG_LOG_LEVEL?.toLowerCase().trim();
  if (Q && Object.hasOwn(mQ, Q)) return Q;
  return "debug";
});
var BU = false;
var l5 = X1(() => {
  return BU || x9(process.env.DEBUG) || x9(process.env.DEBUG_SDK) || process.argv.includes("--debug") || process.argv.includes("-d") || c5() || process.argv.some((Q) => Q.startsWith("--debug=")) || p5() !== null;
});
var zU = X1(() => {
  let Q = process.argv.find((Y) => Y.startsWith("--debug="));
  if (!Q) return null;
  let X = Q.substring(8);
  return k5(X);
});
var c5 = X1(() => {
  return process.argv.includes("--debug-to-stderr") || process.argv.includes("-d2e");
});
var p5 = X1(() => {
  for (let Q = 0; Q < process.argv.length; Q++) {
    let X = process.argv[Q];
    if (X.startsWith("--debug-file=")) return X.substring(13);
    if (X === "--debug-file" && Q + 1 < process.argv.length) return process.argv[Q + 1];
  }
  return null;
});
function KU(Q) {
  if (!l5()) return false;
  if (typeof process > "u" || typeof process.versions > "u" || typeof process.versions.node > "u") return false;
  let X = zU();
  return v5(Q, X);
}
var VU = false;
var n4 = null;
function qU() {
  if (!n4) {
    let Q = null;
    n4 = g5({ writeFn: (X) => {
      let Y = d5(), $ = u5(Y);
      if (Q !== $) {
        try {
          i6().mkdirSync($);
        } catch {
        }
        Q = $;
      }
      i6().appendFileSync(Y, X), UU();
    }, flushIntervalMs: 1e3, maxBufferSize: 100, immediateMode: l5() }), f5(async () => n4?.dispose());
  }
  return n4;
}
function N1(Q, { level: X } = { level: "debug" }) {
  if (mQ[X] < mQ[HU()]) return;
  if (!KU(Q)) return;
  if (VU && Q.includes(`
`)) Q = G0(Q);
  let $ = `${(/* @__PURE__ */ new Date()).toISOString()} [${X.toUpperCase()}] ${Q.trim()}
`;
  if (c5()) {
    _5($);
    return;
  }
  qU().write($);
}
function d5() {
  return p5() ?? process.env.CLAUDE_CODE_DEBUG_LOGS_DIR ?? m5(c6(), "debug", `${y5()}.txt`);
}
var UU = X1(() => {
  try {
    let Q = d5(), X = u5(Q), Y = m5(X, "latest");
    try {
      i6().mkdirSync(X);
    } catch {
    }
    try {
      i6().unlinkSync(Y);
    } catch {
    }
    i6().symlinkSync(Q, Y);
  } catch {
  }
});
var $C = (() => {
  let Q = process.env.CLAUDE_CODE_SLOW_OPERATION_THRESHOLD_MS;
  if (Q !== void 0) {
    let X = Number(Q);
    if (!Number.isNaN(X) && X >= 0) return X;
  }
  return 1 / 0;
})();
var LU = { [Symbol.dispose]() {
} };
function FU() {
  return LU;
}
var U0 = FU;
function G0(Q, X, Y) {
  let J = [];
  try {
    const $ = X0(J, U0`JSON.stringify(${Q})`, 0);
    return JSON.stringify(Q, X, Y);
  } catch (W) {
    var G = W, H = 1;
  } finally {
    Y0(J, G, H);
  }
}
var O1 = (Q, X) => {
  let $ = [];
  try {
    const Y = X0($, U0`JSON.parse(${Q})`, 0);
    return typeof X > "u" ? JSON.parse(Q) : JSON.parse(Q, X);
  } catch (J) {
    var W = J, G = 1;
  } finally {
    Y0($, W, G);
  }
};
function NU(Q) {
  let X = Q.trim();
  return X.startsWith("{") && X.endsWith("}");
}
function i5(Q, X) {
  let Y = { ...Q };
  if (X) {
    let $ = Y.settings;
    if ($ && !NU($)) throw Error("Cannot use both a settings file path and the sandbox option. Include the sandbox configuration in your settings file instead.");
    let J = { sandbox: X };
    if ($) try {
      J = { ...O1($), sandbox: X };
    } catch {
    }
    Y.settings = G0(J);
  }
  return Y;
}
var y9 = class {
  options;
  process;
  processStdin;
  processStdout;
  ready = false;
  abortController;
  exitError;
  exitListeners = [];
  processExitHandler;
  abortHandler;
  constructor(Q) {
    this.options = Q;
    this.abortController = Q.abortController || y6(), this.initialize();
  }
  getDefaultExecutable() {
    return g6() ? "bun" : "node";
  }
  spawnLocalProcess(Q) {
    let { command: X, args: Y, cwd: $, env: J, signal: W } = Q, G = J.DEBUG_CLAUDE_AGENT_SDK || this.options.stderr ? "pipe" : "ignore", H = OU(X, Y, { cwd: $, stdio: ["pipe", "pipe", G], signal: W, env: J, windowsHide: true });
    if (J.DEBUG_CLAUDE_AGENT_SDK || this.options.stderr) H.stderr.on("data", (z) => {
      let K = z.toString();
      if (i0(K), this.options.stderr) this.options.stderr(K);
    });
    return { stdin: H.stdin, stdout: H.stdout, get killed() {
      return H.killed;
    }, get exitCode() {
      return H.exitCode;
    }, kill: H.kill.bind(H), on: H.on.bind(H), once: H.once.bind(H), off: H.off.bind(H) };
  }
  initialize() {
    try {
      let { additionalDirectories: Q = [], agent: X, betas: Y, cwd: $, executable: J = this.getDefaultExecutable(), executableArgs: W = [], extraArgs: G = {}, pathToClaudeCodeExecutable: H, env: B = { ...process.env }, thinkingConfig: z, maxTurns: K, maxBudgetUsd: U, model: q, fallbackModel: V, jsonSchema: L, permissionMode: F, allowDangerouslySkipPermissions: M, permissionPromptToolName: O, continueConversation: A, resume: R, settingSources: Z, allowedTools: C = [], disallowedTools: B0 = [], tools: O0, mcpServers: d0, strictMcpConfig: B6, canUseTool: F1, includePartialMessages: z6, plugins: y1, sandbox: K6 } = this.options, h = ["--output-format", "stream-json", "--verbose", "--input-format", "stream-json"];
      if (z) switch (z.type) {
        case "enabled":
          if (z.budgetTokens === void 0) h.push("--thinking", "adaptive");
          else h.push("--max-thinking-tokens", z.budgetTokens.toString());
          break;
        case "disabled":
          h.push("--thinking", "disabled");
          break;
        case "adaptive":
          h.push("--thinking", "adaptive");
          break;
      }
      if (this.options.effort) h.push("--effort", this.options.effort);
      if (K) h.push("--max-turns", K.toString());
      if (U !== void 0) h.push("--max-budget-usd", U.toString());
      if (q) h.push("--model", q);
      if (X) h.push("--agent", X);
      if (Y && Y.length > 0) h.push("--betas", Y.join(","));
      if (L) h.push("--json-schema", G0(L));
      if (this.options.debugFile) h.push("--debug-file", this.options.debugFile);
      else if (this.options.debug) h.push("--debug");
      if (B.DEBUG_CLAUDE_AGENT_SDK) h.push("--debug-to-stderr");
      if (F1) {
        if (O) throw Error("canUseTool callback cannot be used with permissionPromptToolName. Please use one or the other.");
        h.push("--permission-prompt-tool", "stdio");
      } else if (O) h.push("--permission-prompt-tool", O);
      if (A) h.push("--continue");
      if (R) h.push("--resume", R);
      if (this.options.proactive) h.push("--proactive");
      if (this.options.assistant) h.push("--assistant");
      if (C.length > 0) h.push("--allowedTools", C.join(","));
      if (B0.length > 0) h.push("--disallowedTools", B0.join(","));
      if (O0 !== void 0) if (Array.isArray(O0)) if (O0.length === 0) h.push("--tools", "");
      else h.push("--tools", O0.join(","));
      else h.push("--tools", "default");
      if (d0 && Object.keys(d0).length > 0) h.push("--mcp-config", G0({ mcpServers: d0 }));
      if (Z) h.push("--setting-sources", Z.join(","));
      if (B6) h.push("--strict-mcp-config");
      if (F) h.push("--permission-mode", F);
      if (M) h.push("--allow-dangerously-skip-permissions");
      if (V) {
        if (q && V === q) throw Error("Fallback model cannot be the same as the main model. Please specify a different model for fallbackModel option.");
        h.push("--fallback-model", V);
      }
      if (z6) h.push("--include-partial-messages");
      for (let C0 of Q) h.push("--add-dir", C0);
      if (y1 && y1.length > 0) for (let C0 of y1) if (C0.type === "local") h.push("--plugin-dir", C0.path);
      else throw Error(`Unsupported plugin type: ${C0.type}`);
      if (this.options.forkSession) h.push("--fork-session");
      if (this.options.resumeSessionAt) h.push("--resume-session-at", this.options.resumeSessionAt);
      if (this.options.sessionId) h.push("--session-id", this.options.sessionId);
      if (this.options.persistSession === false) h.push("--no-session-persistence");
      let S9 = { ...G ?? {} };
      if (this.options.settings) S9.settings = this.options.settings;
      let TQ = i5(S9, K6);
      for (let [C0, g1] of Object.entries(TQ)) if (g1 === null) h.push(`--${C0}`);
      else h.push(`--${C0}`, g1);
      if (!B.CLAUDE_CODE_ENTRYPOINT) B.CLAUDE_CODE_ENTRYPOINT = "sdk-ts";
      if (delete B.NODE_OPTIONS, B.DEBUG_CLAUDE_AGENT_SDK) B.DEBUG = "1";
      else delete B.DEBUG;
      let _9 = MU(H), k9 = _9 ? H : J, V6 = _9 ? [...W, ...h] : [...W, H, ...h], l4 = { command: k9, args: V6, cwd: $, env: B, signal: this.abortController.signal };
      if (this.options.spawnClaudeCodeProcess) i0(`Spawning Claude Code (custom): ${k9} ${V6.join(" ")}`), this.process = this.options.spawnClaudeCodeProcess(l4);
      else i0(`Spawning Claude Code: ${k9} ${V6.join(" ")}`), this.process = this.spawnLocalProcess(l4);
      this.processStdin = this.process.stdin, this.processStdout = this.process.stdout;
      let k6 = () => {
        if (this.process && !this.process.killed) this.process.kill("SIGTERM");
      };
      this.processExitHandler = k6, this.abortHandler = k6, process.on("exit", this.processExitHandler), this.abortController.signal.addEventListener("abort", this.abortHandler), this.process.on("error", (C0) => {
        if (this.ready = false, this.abortController.signal.aborted) this.exitError = new T0("Claude Code process aborted by user");
        else if (C0.code === "ENOENT") {
          let g1 = _9 ? `Claude Code native binary not found at ${H}. Please ensure Claude Code is installed via native installer or specify a valid path with options.pathToClaudeCodeExecutable.` : `Claude Code executable not found at ${H}. Is options.pathToClaudeCodeExecutable set?`;
          this.exitError = ReferenceError(g1), i0(this.exitError.message);
        } else this.exitError = Error(`Failed to spawn Claude Code process: ${C0.message}`), i0(this.exitError.message);
      }), this.process.on("exit", (C0, g1) => {
        if (this.ready = false, this.abortController.signal.aborted) this.exitError = new T0("Claude Code process aborted by user");
        else {
          let v6 = this.getProcessExitError(C0, g1);
          if (v6) this.exitError = v6, i0(v6.message);
        }
      }), this.ready = true;
    } catch (Q) {
      throw this.ready = false, Q;
    }
  }
  getProcessExitError(Q, X) {
    if (Q !== 0 && Q !== null) return Error(`Claude Code process exited with code ${Q}`);
    else if (X) return Error(`Claude Code process terminated by signal ${X}`);
    return;
  }
  write(Q) {
    if (this.abortController.signal.aborted) throw new T0("Operation aborted");
    if (!this.ready || !this.processStdin) throw Error("ProcessTransport is not ready for writing");
    if (this.process?.killed || this.process?.exitCode !== null) throw Error("Cannot write to terminated process");
    if (this.exitError) throw Error(`Cannot write to process that exited with error: ${this.exitError.message}`);
    i0(`[ProcessTransport] Writing to stdin: ${Q.substring(0, 100)}`);
    try {
      if (!this.processStdin.write(Q)) i0("[ProcessTransport] Write buffer full, data queued");
    } catch (X) {
      throw this.ready = false, Error(`Failed to write to process stdin: ${X.message}`);
    }
  }
  close() {
    if (this.processStdin) this.processStdin.end(), this.processStdin = void 0;
    if (this.abortHandler) this.abortController.signal.removeEventListener("abort", this.abortHandler), this.abortHandler = void 0;
    for (let { handler: Q } of this.exitListeners) this.process?.off("exit", Q);
    if (this.exitListeners = [], this.process && !this.process.killed && this.process.exitCode === null) this.process.kill("SIGTERM"), setTimeout(() => {
      if (this.process && !this.process.killed) this.process.kill("SIGKILL");
    }, 5e3).unref();
    if (this.ready = false, this.processExitHandler) process.off("exit", this.processExitHandler), this.processExitHandler = void 0;
  }
  isReady() {
    return this.ready;
  }
  async *readMessages() {
    if (!this.processStdout) throw Error("ProcessTransport output stream not available");
    let Q = DU({ input: this.processStdout });
    try {
      for await (let X of Q) if (X.trim()) try {
        yield O1(X);
      } catch (Y) {
        throw i0(`Non-JSON stdout: ${X}`), Error(`CLI output was not valid JSON. This may indicate an error during startup. Output: ${X.slice(0, 200)}${X.length > 200 ? "..." : ""}`);
      }
      await this.waitForExit();
    } catch (X) {
      throw X;
    } finally {
      Q.close();
    }
  }
  endInput() {
    if (this.processStdin) this.processStdin.end();
  }
  getInputStream() {
    return this.processStdin;
  }
  onExit(Q) {
    if (!this.process) return () => {
    };
    let X = (Y, $) => {
      let J = this.getProcessExitError(Y, $);
      Q(J);
    };
    return this.process.on("exit", X), this.exitListeners.push({ callback: Q, handler: X }), () => {
      if (this.process) this.process.off("exit", X);
      let Y = this.exitListeners.findIndex(($) => $.handler === X);
      if (Y !== -1) this.exitListeners.splice(Y, 1);
    };
  }
  async waitForExit() {
    if (!this.process) {
      if (this.exitError) throw this.exitError;
      return;
    }
    if (this.process.exitCode !== null || this.process.killed) {
      if (this.exitError) throw this.exitError;
      return;
    }
    return new Promise((Q, X) => {
      let Y = (J, W) => {
        if (this.abortController.signal.aborted) {
          X(new T0("Operation aborted"));
          return;
        }
        let G = this.getProcessExitError(J, W);
        if (G) X(G);
        else Q();
      };
      this.process.once("exit", Y);
      let $ = (J) => {
        this.process.off("exit", Y), X(J);
      };
      this.process.once("error", $), this.process.once("exit", () => {
        this.process.off("error", $);
      });
    });
  }
};
function MU(Q) {
  return ![".js", ".mjs", ".tsx", ".ts", ".jsx"].some((Y) => Q.endsWith(Y));
}
var g9 = class {
  returned;
  queue = [];
  readResolve;
  readReject;
  isDone = false;
  hasError;
  started = false;
  constructor(Q) {
    this.returned = Q;
  }
  [Symbol.asyncIterator]() {
    if (this.started) throw Error("Stream can only be iterated once");
    return this.started = true, this;
  }
  next() {
    if (this.queue.length > 0) return Promise.resolve({ done: false, value: this.queue.shift() });
    if (this.isDone) return Promise.resolve({ done: true, value: void 0 });
    if (this.hasError) return Promise.reject(this.hasError);
    return new Promise((Q, X) => {
      this.readResolve = Q, this.readReject = X;
    });
  }
  enqueue(Q) {
    if (this.readResolve) {
      let X = this.readResolve;
      this.readResolve = void 0, this.readReject = void 0, X({ done: false, value: Q });
    } else this.queue.push(Q);
  }
  done() {
    if (this.isDone = true, this.readResolve) {
      let Q = this.readResolve;
      this.readResolve = void 0, this.readReject = void 0, Q({ done: true, value: void 0 });
    }
  }
  error(Q) {
    if (this.hasError = Q, this.readReject) {
      let X = this.readReject;
      this.readResolve = void 0, this.readReject = void 0, X(Q);
    }
  }
  return() {
    if (this.isDone = true, this.returned) this.returned();
    return Promise.resolve({ done: true, value: void 0 });
  }
};
var lQ = class {
  sendMcpMessage;
  isClosed = false;
  constructor(Q) {
    this.sendMcpMessage = Q;
  }
  onclose;
  onerror;
  onmessage;
  async start() {
  }
  async send(Q) {
    if (this.isClosed) throw Error("Transport is closed");
    this.sendMcpMessage(Q);
  }
  async close() {
    if (this.isClosed) return;
    this.isClosed = true, this.onclose?.();
  }
};
var h9 = class {
  transport;
  isSingleUserTurn;
  canUseTool;
  hooks;
  abortController;
  jsonSchema;
  initConfig;
  onElicitation;
  pendingControlResponses = /* @__PURE__ */ new Map();
  cleanupPerformed = false;
  sdkMessages;
  inputStream = new g9();
  initialization;
  cancelControllers = /* @__PURE__ */ new Map();
  hookCallbacks = /* @__PURE__ */ new Map();
  nextCallbackId = 0;
  sdkMcpTransports = /* @__PURE__ */ new Map();
  sdkMcpServerInstances = /* @__PURE__ */ new Map();
  pendingMcpResponses = /* @__PURE__ */ new Map();
  firstResultReceivedResolve;
  firstResultReceived = false;
  lastErrorResultText;
  hasBidirectionalNeeds() {
    return this.sdkMcpTransports.size > 0 || this.hooks !== void 0 && Object.keys(this.hooks).length > 0 || this.canUseTool !== void 0 || this.onElicitation !== void 0;
  }
  constructor(Q, X, Y, $, J, W = /* @__PURE__ */ new Map(), G, H, B) {
    this.transport = Q;
    this.isSingleUserTurn = X;
    this.canUseTool = Y;
    this.hooks = $;
    this.abortController = J;
    this.jsonSchema = G;
    this.initConfig = H;
    this.onElicitation = B;
    for (let [z, K] of W) this.connectSdkMcpServer(z, K);
    this.sdkMessages = this.readSdkMessages(), this.readMessages(), this.initialization = this.initialize(), this.initialization.catch(() => {
    });
  }
  setError(Q) {
    this.inputStream.error(Q);
  }
  async stopTask(Q) {
    await this.request({ subtype: "stop_task", task_id: Q });
  }
  close() {
    this.cleanup();
  }
  cleanup(Q) {
    if (this.cleanupPerformed) return;
    this.cleanupPerformed = true;
    try {
      this.transport.close();
      let X = Error("Query closed before response received");
      for (let { reject: Y } of this.pendingControlResponses.values()) Y(X);
      this.pendingControlResponses.clear();
      for (let { reject: Y } of this.pendingMcpResponses.values()) Y(X);
      this.pendingMcpResponses.clear(), this.cancelControllers.clear(), this.hookCallbacks.clear();
      for (let Y of this.sdkMcpTransports.values()) try {
        Y.close();
      } catch {
      }
      if (this.sdkMcpTransports.clear(), Q) this.inputStream.error(Q);
      else this.inputStream.done();
    } catch (X) {
    }
  }
  next(...[Q]) {
    return this.sdkMessages.next(...[Q]);
  }
  return(Q) {
    return this.sdkMessages.return(Q);
  }
  throw(Q) {
    return this.sdkMessages.throw(Q);
  }
  [Symbol.asyncIterator]() {
    return this.sdkMessages;
  }
  [Symbol.asyncDispose]() {
    return this.sdkMessages[Symbol.asyncDispose]();
  }
  async readMessages() {
    try {
      for await (let Q of this.transport.readMessages()) {
        if (Q.type === "control_response") {
          let X = this.pendingControlResponses.get(Q.response.request_id);
          if (X) X.handler(Q.response);
          continue;
        } else if (Q.type === "control_request") {
          this.handleControlRequest(Q);
          continue;
        } else if (Q.type === "control_cancel_request") {
          this.handleControlCancelRequest(Q);
          continue;
        } else if (Q.type === "keep_alive") continue;
        if (Q.type === "streamlined_text" || Q.type === "streamlined_tool_use_summary") continue;
        if (Q.type === "result") {
          if (this.lastErrorResultText = Q.is_error ? Q.subtype === "success" ? Q.result : Q.errors.join("; ") : void 0, this.firstResultReceived = true, this.firstResultReceivedResolve) this.firstResultReceivedResolve();
          if (this.isSingleUserTurn) N1("[Query.readMessages] First result received for single-turn query, closing stdin"), this.transport.endInput();
        } else this.lastErrorResultText = void 0;
        this.inputStream.enqueue(Q);
      }
      if (this.firstResultReceivedResolve) this.firstResultReceivedResolve();
      this.inputStream.done(), this.cleanup();
    } catch (Q) {
      if (this.firstResultReceivedResolve) this.firstResultReceivedResolve();
      if (this.lastErrorResultText !== void 0 && !(Q instanceof T0)) {
        let X = Error(`Claude Code returned an error result: ${this.lastErrorResultText}`);
        N1(`[Query.readMessages] Replacing exit error with result text. Original: ${Q.message}`), this.inputStream.error(X), this.cleanup(X);
        return;
      }
      this.inputStream.error(Q), this.cleanup(Q);
    }
  }
  async handleControlRequest(Q) {
    let X = new AbortController();
    this.cancelControllers.set(Q.request_id, X);
    try {
      let Y = await this.processControlRequest(Q, X.signal), $ = { type: "control_response", response: { subtype: "success", request_id: Q.request_id, response: Y } };
      await Promise.resolve(this.transport.write(G0($) + `
`));
    } catch (Y) {
      let $ = { type: "control_response", response: { subtype: "error", request_id: Q.request_id, error: Y.message || String(Y) } };
      await Promise.resolve(this.transport.write(G0($) + `
`));
    } finally {
      this.cancelControllers.delete(Q.request_id);
    }
  }
  handleControlCancelRequest(Q) {
    let X = this.cancelControllers.get(Q.request_id);
    if (X) X.abort(), this.cancelControllers.delete(Q.request_id);
  }
  async processControlRequest(Q, X) {
    if (Q.request.subtype === "can_use_tool") {
      if (!this.canUseTool) throw Error("canUseTool callback is not provided.");
      return { ...await this.canUseTool(Q.request.tool_name, Q.request.input, { signal: X, suggestions: Q.request.permission_suggestions, blockedPath: Q.request.blocked_path, decisionReason: Q.request.decision_reason, toolUseID: Q.request.tool_use_id, agentID: Q.request.agent_id }), toolUseID: Q.request.tool_use_id };
    } else if (Q.request.subtype === "hook_callback") return await this.handleHookCallbacks(Q.request.callback_id, Q.request.input, Q.request.tool_use_id, X);
    else if (Q.request.subtype === "mcp_message") {
      let Y = Q.request, $ = this.sdkMcpTransports.get(Y.server_name);
      if (!$) throw Error(`SDK MCP server not found: ${Y.server_name}`);
      if ("method" in Y.message && "id" in Y.message && Y.message.id !== null) return { mcp_response: await this.handleMcpControlRequest(Y.server_name, Y, $) };
      else {
        if ($.onmessage) $.onmessage(Y.message);
        return { mcp_response: { jsonrpc: "2.0", result: {}, id: 0 } };
      }
    } else if (Q.request.subtype === "elicitation") {
      let Y = Q.request;
      if (this.onElicitation) return await this.onElicitation({ serverName: Y.mcp_server_name, message: Y.message, mode: Y.mode, url: Y.url, elicitationId: Y.elicitation_id, requestedSchema: Y.requested_schema }, { signal: X });
      return { action: "decline" };
    }
    throw Error("Unsupported control request subtype: " + Q.request.subtype);
  }
  async *readSdkMessages() {
    for await (let Q of this.inputStream) yield Q;
  }
  async initialize() {
    let Q;
    if (this.hooks) {
      Q = {};
      for (let [J, W] of Object.entries(this.hooks)) if (W.length > 0) Q[J] = W.map((G) => {
        let H = [];
        for (let B of G.hooks) {
          let z = `hook_${this.nextCallbackId++}`;
          this.hookCallbacks.set(z, B), H.push(z);
        }
        return { matcher: G.matcher, hookCallbackIds: H, timeout: G.timeout };
      });
    }
    let X = this.sdkMcpTransports.size > 0 ? Array.from(this.sdkMcpTransports.keys()) : void 0, Y = { subtype: "initialize", hooks: Q, sdkMcpServers: X, jsonSchema: this.jsonSchema, systemPrompt: this.initConfig?.systemPrompt, appendSystemPrompt: this.initConfig?.appendSystemPrompt, agents: this.initConfig?.agents, promptSuggestions: this.initConfig?.promptSuggestions, agentProgressSummaries: this.initConfig?.agentProgressSummaries };
    return (await this.request(Y)).response;
  }
  async interrupt() {
    await this.request({ subtype: "interrupt" });
  }
  async setPermissionMode(Q) {
    await this.request({ subtype: "set_permission_mode", mode: Q });
  }
  async setModel(Q) {
    await this.request({ subtype: "set_model", model: Q });
  }
  async setMaxThinkingTokens(Q) {
    await this.request({ subtype: "set_max_thinking_tokens", max_thinking_tokens: Q });
  }
  async applyFlagSettings(Q) {
    await this.request({ subtype: "apply_flag_settings", settings: Q });
  }
  async getSettings() {
    return (await this.request({ subtype: "get_settings" })).response;
  }
  async rewindFiles(Q, X) {
    return (await this.request({ subtype: "rewind_files", user_message_id: Q, dry_run: X?.dryRun })).response;
  }
  async cancelAsyncMessage(Q) {
    return (await this.request({ subtype: "cancel_async_message", message_uuid: Q })).response.cancelled;
  }
  async enableRemoteControl(Q) {
    return (await this.request({ subtype: "remote_control", enabled: Q })).response;
  }
  async setProactive(Q) {
    await this.request({ subtype: "set_proactive", enabled: Q });
  }
  async generateSessionTitle(Q, X) {
    return (await this.request({ subtype: "generate_session_title", description: Q, persist: X?.persist })).response.title;
  }
  async processPendingPermissionRequests(Q) {
    for (let X of Q) if (X.request.subtype === "can_use_tool") this.handleControlRequest(X).catch(() => {
    });
  }
  request(Q) {
    let X = Math.random().toString(36).substring(2, 15), Y = { request_id: X, type: "control_request", request: Q };
    return new Promise(($, J) => {
      this.pendingControlResponses.set(X, { handler: (W) => {
        if (this.pendingControlResponses.delete(X), W.subtype === "success") $(W);
        else if (J(Error(W.error)), W.pending_permission_requests) this.processPendingPermissionRequests(W.pending_permission_requests);
      }, reject: J }), Promise.resolve(this.transport.write(G0(Y) + `
`));
    });
  }
  async initializationResult() {
    return this.initialization;
  }
  async supportedCommands() {
    return (await this.initialization).commands;
  }
  async supportedModels() {
    return (await this.initialization).models;
  }
  async supportedAgents() {
    return (await this.initialization).agents;
  }
  async reconnectMcpServer(Q) {
    await this.request({ subtype: "mcp_reconnect", serverName: Q });
  }
  async toggleMcpServer(Q, X) {
    await this.request({ subtype: "mcp_toggle", serverName: Q, enabled: X });
  }
  async mcpAuthenticate(Q) {
    return (await this.request({ subtype: "mcp_authenticate", serverName: Q })).response;
  }
  async mcpClearAuth(Q) {
    return (await this.request({ subtype: "mcp_clear_auth", serverName: Q })).response;
  }
  async mcpSubmitOAuthCallbackUrl(Q, X) {
    return (await this.request({ subtype: "mcp_oauth_callback_url", serverName: Q, callbackUrl: X })).response;
  }
  async mcpServerStatus() {
    return (await this.request({ subtype: "mcp_status" })).response.mcpServers;
  }
  async setMcpServers(Q) {
    let X = {}, Y = {};
    for (let [H, B] of Object.entries(Q)) if (B.type === "sdk" && "instance" in B) X[H] = B.instance;
    else Y[H] = B;
    let $ = new Set(this.sdkMcpServerInstances.keys()), J = new Set(Object.keys(X));
    for (let H of $) if (!J.has(H)) await this.disconnectSdkMcpServer(H);
    for (let [H, B] of Object.entries(X)) if (!$.has(H)) this.connectSdkMcpServer(H, B);
    let W = {};
    for (let H of Object.keys(X)) W[H] = { type: "sdk", name: H };
    return (await this.request({ subtype: "mcp_set_servers", servers: { ...Y, ...W } })).response;
  }
  async accountInfo() {
    return (await this.initialization).account;
  }
  async streamInput(Q) {
    N1("[Query.streamInput] Starting to process input stream");
    try {
      let X = 0;
      for await (let Y of Q) {
        if (X++, N1(`[Query.streamInput] Processing message ${X}: ${Y.type}`), this.abortController?.signal.aborted) break;
        await Promise.resolve(this.transport.write(G0(Y) + `
`));
      }
      if (N1(`[Query.streamInput] Finished processing ${X} messages from input stream`), X > 0 && this.hasBidirectionalNeeds()) N1("[Query.streamInput] Has bidirectional needs, waiting for first result"), await this.waitForFirstResult();
      N1("[Query] Calling transport.endInput() to close stdin to CLI process"), this.transport.endInput();
    } catch (X) {
      if (!(X instanceof T0)) throw X;
    }
  }
  waitForFirstResult() {
    if (this.firstResultReceived) return N1("[Query.waitForFirstResult] Result already received, returning immediately"), Promise.resolve();
    return new Promise((Q) => {
      if (this.abortController?.signal.aborted) {
        Q();
        return;
      }
      this.abortController?.signal.addEventListener("abort", () => Q(), { once: true }), this.firstResultReceivedResolve = Q;
    });
  }
  handleHookCallbacks(Q, X, Y, $) {
    let J = this.hookCallbacks.get(Q);
    if (!J) throw Error(`No hook callback found for ID: ${Q}`);
    return J(X, Y, { signal: $ });
  }
  connectSdkMcpServer(Q, X) {
    let Y = new lQ(($) => this.sendMcpServerMessageToCli(Q, $));
    this.sdkMcpTransports.set(Q, Y), this.sdkMcpServerInstances.set(Q, X), X.connect(Y);
  }
  async disconnectSdkMcpServer(Q) {
    let X = this.sdkMcpTransports.get(Q);
    if (X) await X.close(), this.sdkMcpTransports.delete(Q);
    this.sdkMcpServerInstances.delete(Q);
  }
  sendMcpServerMessageToCli(Q, X) {
    if ("id" in X && X.id !== null && X.id !== void 0) {
      let $ = `${Q}:${X.id}`, J = this.pendingMcpResponses.get($);
      if (J) {
        J.resolve(X), this.pendingMcpResponses.delete($);
        return;
      }
    }
    let Y = { type: "control_request", request_id: i4(), request: { subtype: "mcp_message", server_name: Q, message: X } };
    this.transport.write(G0(Y) + `
`);
  }
  handleMcpControlRequest(Q, X, Y) {
    let $ = "id" in X.message ? X.message.id : null, J = `${Q}:${$}`;
    return new Promise((W, G) => {
      let H = () => {
        this.pendingMcpResponses.delete(J);
      }, B = (K) => {
        H(), W(K);
      }, z = (K) => {
        H(), G(K);
      };
      if (this.pendingMcpResponses.set(J, { resolve: B, reject: z }), Y.onmessage) Y.onmessage(X.message);
      else {
        H(), G(Error("No message handler registered"));
        return;
      }
    });
  }
};
var IU = RU(jU);
var i1 = {};
gQ(i1, { void: () => uL, util: () => d, unknown: () => hL, union: () => cL, undefined: () => xL, tuple: () => iL, transformer: () => YF, symbol: () => TL, string: () => MJ, strictObject: () => lL, setErrorMap: () => WL, set: () => rL, record: () => nL, quotelessJson: () => $L, promise: () => XF, preprocess: () => WF, pipeline: () => GF, ostring: () => HF, optional: () => $F, onumber: () => BF, oboolean: () => zF, objectUtil: () => eQ, object: () => YX, number: () => wJ, nullable: () => JF, null: () => yL, never: () => fL, nativeEnum: () => QF, nan: () => _L, map: () => oL, makeIssue: () => m9, literal: () => sL, lazy: () => aL, late: () => CL, isValid: () => m1, isDirty: () => X8, isAsync: () => t6, isAborted: () => Q8, intersection: () => dL, instanceof: () => SL, getParsedType: () => M1, getErrorMap: () => r6, function: () => tL, enum: () => eL, effect: () => YF, discriminatedUnion: () => pL, defaultErrorMap: () => Z1, datetimeRegex: () => NJ, date: () => vL, custom: () => DJ, coerce: () => KF, boolean: () => AJ, bigint: () => kL, array: () => mL, any: () => gL, addIssueToContext: () => b, ZodVoid: () => c9, ZodUnknown: () => l1, ZodUnion: () => Y9, ZodUndefined: () => Q9, ZodType: () => p, ZodTuple: () => A1, ZodTransformer: () => W1, ZodSymbol: () => l9, ZodString: () => Y1, ZodSet: () => O6, ZodSchema: () => p, ZodRecord: () => p9, ZodReadonly: () => z9, ZodPromise: () => D6, ZodPipeline: () => n9, ZodParsedType: () => I, ZodOptional: () => m0, ZodObject: () => L0, ZodNumber: () => c1, ZodNullable: () => S1, ZodNull: () => X9, ZodNever: () => w1, ZodNativeEnum: () => G9, ZodNaN: () => i9, ZodMap: () => d9, ZodLiteral: () => W9, ZodLazy: () => J9, ZodIssueCode: () => w, ZodIntersection: () => $9, ZodFunction: () => s6, ZodFirstPartyTypeKind: () => j, ZodError: () => x0, ZodEnum: () => d1, ZodEffects: () => W1, ZodDiscriminatedUnion: () => Y8, ZodDefault: () => H9, ZodDate: () => F6, ZodCatch: () => B9, ZodBranded: () => $8, ZodBoolean: () => e6, ZodBigInt: () => p1, ZodArray: () => $1, ZodAny: () => N6, Schema: () => p, ParseStatus: () => A0, OK: () => P0, NEVER: () => VF, INVALID: () => x, EMPTY_PATH: () => GL, DIRTY: () => L6, BRAND: () => ZL });
var d;
(function(Q) {
  Q.assertEqual = (J) => {
  };
  function X(J) {
  }
  Q.assertIs = X;
  function Y(J) {
    throw Error();
  }
  Q.assertNever = Y, Q.arrayToEnum = (J) => {
    let W = {};
    for (let G of J) W[G] = G;
    return W;
  }, Q.getValidEnumValues = (J) => {
    let W = Q.objectKeys(J).filter((H) => typeof J[J[H]] !== "number"), G = {};
    for (let H of W) G[H] = J[H];
    return Q.objectValues(G);
  }, Q.objectValues = (J) => {
    return Q.objectKeys(J).map(function(W) {
      return J[W];
    });
  }, Q.objectKeys = typeof Object.keys === "function" ? (J) => Object.keys(J) : (J) => {
    let W = [];
    for (let G in J) if (Object.prototype.hasOwnProperty.call(J, G)) W.push(G);
    return W;
  }, Q.find = (J, W) => {
    for (let G of J) if (W(G)) return G;
    return;
  }, Q.isInteger = typeof Number.isInteger === "function" ? (J) => Number.isInteger(J) : (J) => typeof J === "number" && Number.isFinite(J) && Math.floor(J) === J;
  function $(J, W = " | ") {
    return J.map((G) => typeof G === "string" ? `'${G}'` : G).join(W);
  }
  Q.joinValues = $, Q.jsonStringifyReplacer = (J, W) => {
    if (typeof W === "bigint") return W.toString();
    return W;
  };
})(d || (d = {}));
var eQ;
(function(Q) {
  Q.mergeShapes = (X, Y) => {
    return { ...X, ...Y };
  };
})(eQ || (eQ = {}));
var I = d.arrayToEnum(["string", "nan", "number", "integer", "float", "boolean", "date", "bigint", "symbol", "function", "undefined", "null", "array", "object", "unknown", "promise", "void", "never", "map", "set"]);
var M1 = (Q) => {
  switch (typeof Q) {
    case "undefined":
      return I.undefined;
    case "string":
      return I.string;
    case "number":
      return Number.isNaN(Q) ? I.nan : I.number;
    case "boolean":
      return I.boolean;
    case "function":
      return I.function;
    case "bigint":
      return I.bigint;
    case "symbol":
      return I.symbol;
    case "object":
      if (Array.isArray(Q)) return I.array;
      if (Q === null) return I.null;
      if (Q.then && typeof Q.then === "function" && Q.catch && typeof Q.catch === "function") return I.promise;
      if (typeof Map < "u" && Q instanceof Map) return I.map;
      if (typeof Set < "u" && Q instanceof Set) return I.set;
      if (typeof Date < "u" && Q instanceof Date) return I.date;
      return I.object;
    default:
      return I.unknown;
  }
};
var w = d.arrayToEnum(["invalid_type", "invalid_literal", "custom", "invalid_union", "invalid_union_discriminator", "invalid_enum_value", "unrecognized_keys", "invalid_arguments", "invalid_return_type", "invalid_date", "invalid_string", "too_small", "too_big", "invalid_intersection_types", "not_multiple_of", "not_finite"]);
var $L = (Q) => {
  return JSON.stringify(Q, null, 2).replace(/"([^"]+)":/g, "$1:");
};
var x0 = class _x0 extends Error {
  get errors() {
    return this.issues;
  }
  constructor(Q) {
    super();
    this.issues = [], this.addIssue = (Y) => {
      this.issues = [...this.issues, Y];
    }, this.addIssues = (Y = []) => {
      this.issues = [...this.issues, ...Y];
    };
    let X = new.target.prototype;
    if (Object.setPrototypeOf) Object.setPrototypeOf(this, X);
    else this.__proto__ = X;
    this.name = "ZodError", this.issues = Q;
  }
  format(Q) {
    let X = Q || function(J) {
      return J.message;
    }, Y = { _errors: [] }, $ = (J) => {
      for (let W of J.issues) if (W.code === "invalid_union") W.unionErrors.map($);
      else if (W.code === "invalid_return_type") $(W.returnTypeError);
      else if (W.code === "invalid_arguments") $(W.argumentsError);
      else if (W.path.length === 0) Y._errors.push(X(W));
      else {
        let G = Y, H = 0;
        while (H < W.path.length) {
          let B = W.path[H];
          if (H !== W.path.length - 1) G[B] = G[B] || { _errors: [] };
          else G[B] = G[B] || { _errors: [] }, G[B]._errors.push(X(W));
          G = G[B], H++;
        }
      }
    };
    return $(this), Y;
  }
  static assert(Q) {
    if (!(Q instanceof _x0)) throw Error(`Not a ZodError: ${Q}`);
  }
  toString() {
    return this.message;
  }
  get message() {
    return JSON.stringify(this.issues, d.jsonStringifyReplacer, 2);
  }
  get isEmpty() {
    return this.issues.length === 0;
  }
  flatten(Q = (X) => X.message) {
    let X = {}, Y = [];
    for (let $ of this.issues) if ($.path.length > 0) {
      let J = $.path[0];
      X[J] = X[J] || [], X[J].push(Q($));
    } else Y.push(Q($));
    return { formErrors: Y, fieldErrors: X };
  }
  get formErrors() {
    return this.flatten();
  }
};
x0.create = (Q) => {
  return new x0(Q);
};
var JL = (Q, X) => {
  let Y;
  switch (Q.code) {
    case w.invalid_type:
      if (Q.received === I.undefined) Y = "Required";
      else Y = `Expected ${Q.expected}, received ${Q.received}`;
      break;
    case w.invalid_literal:
      Y = `Invalid literal value, expected ${JSON.stringify(Q.expected, d.jsonStringifyReplacer)}`;
      break;
    case w.unrecognized_keys:
      Y = `Unrecognized key(s) in object: ${d.joinValues(Q.keys, ", ")}`;
      break;
    case w.invalid_union:
      Y = "Invalid input";
      break;
    case w.invalid_union_discriminator:
      Y = `Invalid discriminator value. Expected ${d.joinValues(Q.options)}`;
      break;
    case w.invalid_enum_value:
      Y = `Invalid enum value. Expected ${d.joinValues(Q.options)}, received '${Q.received}'`;
      break;
    case w.invalid_arguments:
      Y = "Invalid function arguments";
      break;
    case w.invalid_return_type:
      Y = "Invalid function return type";
      break;
    case w.invalid_date:
      Y = "Invalid date";
      break;
    case w.invalid_string:
      if (typeof Q.validation === "object") if ("includes" in Q.validation) {
        if (Y = `Invalid input: must include "${Q.validation.includes}"`, typeof Q.validation.position === "number") Y = `${Y} at one or more positions greater than or equal to ${Q.validation.position}`;
      } else if ("startsWith" in Q.validation) Y = `Invalid input: must start with "${Q.validation.startsWith}"`;
      else if ("endsWith" in Q.validation) Y = `Invalid input: must end with "${Q.validation.endsWith}"`;
      else d.assertNever(Q.validation);
      else if (Q.validation !== "regex") Y = `Invalid ${Q.validation}`;
      else Y = "Invalid";
      break;
    case w.too_small:
      if (Q.type === "array") Y = `Array must contain ${Q.exact ? "exactly" : Q.inclusive ? "at least" : "more than"} ${Q.minimum} element(s)`;
      else if (Q.type === "string") Y = `String must contain ${Q.exact ? "exactly" : Q.inclusive ? "at least" : "over"} ${Q.minimum} character(s)`;
      else if (Q.type === "number") Y = `Number must be ${Q.exact ? "exactly equal to " : Q.inclusive ? "greater than or equal to " : "greater than "}${Q.minimum}`;
      else if (Q.type === "bigint") Y = `Number must be ${Q.exact ? "exactly equal to " : Q.inclusive ? "greater than or equal to " : "greater than "}${Q.minimum}`;
      else if (Q.type === "date") Y = `Date must be ${Q.exact ? "exactly equal to " : Q.inclusive ? "greater than or equal to " : "greater than "}${new Date(Number(Q.minimum))}`;
      else Y = "Invalid input";
      break;
    case w.too_big:
      if (Q.type === "array") Y = `Array must contain ${Q.exact ? "exactly" : Q.inclusive ? "at most" : "less than"} ${Q.maximum} element(s)`;
      else if (Q.type === "string") Y = `String must contain ${Q.exact ? "exactly" : Q.inclusive ? "at most" : "under"} ${Q.maximum} character(s)`;
      else if (Q.type === "number") Y = `Number must be ${Q.exact ? "exactly" : Q.inclusive ? "less than or equal to" : "less than"} ${Q.maximum}`;
      else if (Q.type === "bigint") Y = `BigInt must be ${Q.exact ? "exactly" : Q.inclusive ? "less than or equal to" : "less than"} ${Q.maximum}`;
      else if (Q.type === "date") Y = `Date must be ${Q.exact ? "exactly" : Q.inclusive ? "smaller than or equal to" : "smaller than"} ${new Date(Number(Q.maximum))}`;
      else Y = "Invalid input";
      break;
    case w.custom:
      Y = "Invalid input";
      break;
    case w.invalid_intersection_types:
      Y = "Intersection results could not be merged";
      break;
    case w.not_multiple_of:
      Y = `Number must be a multiple of ${Q.multipleOf}`;
      break;
    case w.not_finite:
      Y = "Number must be finite";
      break;
    default:
      Y = X.defaultError, d.assertNever(Q);
  }
  return { message: Y };
};
var Z1 = JL;
var VJ = Z1;
function WL(Q) {
  VJ = Q;
}
function r6() {
  return VJ;
}
var m9 = (Q) => {
  let { data: X, path: Y, errorMaps: $, issueData: J } = Q, W = [...Y, ...J.path || []], G = { ...J, path: W };
  if (J.message !== void 0) return { ...J, path: W, message: J.message };
  let H = "", B = $.filter((z) => !!z).slice().reverse();
  for (let z of B) H = z(G, { data: X, defaultError: H }).message;
  return { ...J, path: W, message: H };
};
var GL = [];
function b(Q, X) {
  let Y = r6(), $ = m9({ issueData: X, data: Q.data, path: Q.path, errorMaps: [Q.common.contextualErrorMap, Q.schemaErrorMap, Y, Y === Z1 ? void 0 : Z1].filter((J) => !!J) });
  Q.common.issues.push($);
}
var A0 = class _A0 {
  constructor() {
    this.value = "valid";
  }
  dirty() {
    if (this.value === "valid") this.value = "dirty";
  }
  abort() {
    if (this.value !== "aborted") this.value = "aborted";
  }
  static mergeArray(Q, X) {
    let Y = [];
    for (let $ of X) {
      if ($.status === "aborted") return x;
      if ($.status === "dirty") Q.dirty();
      Y.push($.value);
    }
    return { status: Q.value, value: Y };
  }
  static async mergeObjectAsync(Q, X) {
    let Y = [];
    for (let $ of X) {
      let J = await $.key, W = await $.value;
      Y.push({ key: J, value: W });
    }
    return _A0.mergeObjectSync(Q, Y);
  }
  static mergeObjectSync(Q, X) {
    let Y = {};
    for (let $ of X) {
      let { key: J, value: W } = $;
      if (J.status === "aborted") return x;
      if (W.status === "aborted") return x;
      if (J.status === "dirty") Q.dirty();
      if (W.status === "dirty") Q.dirty();
      if (J.value !== "__proto__" && (typeof W.value < "u" || $.alwaysSet)) Y[J.value] = W.value;
    }
    return { status: Q.value, value: Y };
  }
};
var x = Object.freeze({ status: "aborted" });
var L6 = (Q) => ({ status: "dirty", value: Q });
var P0 = (Q) => ({ status: "valid", value: Q });
var Q8 = (Q) => Q.status === "aborted";
var X8 = (Q) => Q.status === "dirty";
var m1 = (Q) => Q.status === "valid";
var t6 = (Q) => typeof Promise < "u" && Q instanceof Promise;
var S;
(function(Q) {
  Q.errToObj = (X) => typeof X === "string" ? { message: X } : X || {}, Q.toString = (X) => typeof X === "string" ? X : X?.message;
})(S || (S = {}));
var J1 = class {
  constructor(Q, X, Y, $) {
    this._cachedPath = [], this.parent = Q, this.data = X, this._path = Y, this._key = $;
  }
  get path() {
    if (!this._cachedPath.length) if (Array.isArray(this._key)) this._cachedPath.push(...this._path, ...this._key);
    else this._cachedPath.push(...this._path, this._key);
    return this._cachedPath;
  }
};
var qJ = (Q, X) => {
  if (m1(X)) return { success: true, data: X.value };
  else {
    if (!Q.common.issues.length) throw Error("Validation failed but no issues detected.");
    return { success: false, get error() {
      if (this._error) return this._error;
      let Y = new x0(Q.common.issues);
      return this._error = Y, this._error;
    } };
  }
};
function m(Q) {
  if (!Q) return {};
  let { errorMap: X, invalid_type_error: Y, required_error: $, description: J } = Q;
  if (X && (Y || $)) throw Error(`Can't use "invalid_type_error" or "required_error" in conjunction with custom error map.`);
  if (X) return { errorMap: X, description: J };
  return { errorMap: (G, H) => {
    let { message: B } = Q;
    if (G.code === "invalid_enum_value") return { message: B ?? H.defaultError };
    if (typeof H.data > "u") return { message: B ?? $ ?? H.defaultError };
    if (G.code !== "invalid_type") return { message: H.defaultError };
    return { message: B ?? Y ?? H.defaultError };
  }, description: J };
}
var p = class {
  get description() {
    return this._def.description;
  }
  _getType(Q) {
    return M1(Q.data);
  }
  _getOrReturnCtx(Q, X) {
    return X || { common: Q.parent.common, data: Q.data, parsedType: M1(Q.data), schemaErrorMap: this._def.errorMap, path: Q.path, parent: Q.parent };
  }
  _processInputParams(Q) {
    return { status: new A0(), ctx: { common: Q.parent.common, data: Q.data, parsedType: M1(Q.data), schemaErrorMap: this._def.errorMap, path: Q.path, parent: Q.parent } };
  }
  _parseSync(Q) {
    let X = this._parse(Q);
    if (t6(X)) throw Error("Synchronous parse encountered promise.");
    return X;
  }
  _parseAsync(Q) {
    let X = this._parse(Q);
    return Promise.resolve(X);
  }
  parse(Q, X) {
    let Y = this.safeParse(Q, X);
    if (Y.success) return Y.data;
    throw Y.error;
  }
  safeParse(Q, X) {
    let Y = { common: { issues: [], async: X?.async ?? false, contextualErrorMap: X?.errorMap }, path: X?.path || [], schemaErrorMap: this._def.errorMap, parent: null, data: Q, parsedType: M1(Q) }, $ = this._parseSync({ data: Q, path: Y.path, parent: Y });
    return qJ(Y, $);
  }
  "~validate"(Q) {
    let X = { common: { issues: [], async: !!this["~standard"].async }, path: [], schemaErrorMap: this._def.errorMap, parent: null, data: Q, parsedType: M1(Q) };
    if (!this["~standard"].async) try {
      let Y = this._parseSync({ data: Q, path: [], parent: X });
      return m1(Y) ? { value: Y.value } : { issues: X.common.issues };
    } catch (Y) {
      if (Y?.message?.toLowerCase()?.includes("encountered")) this["~standard"].async = true;
      X.common = { issues: [], async: true };
    }
    return this._parseAsync({ data: Q, path: [], parent: X }).then((Y) => m1(Y) ? { value: Y.value } : { issues: X.common.issues });
  }
  async parseAsync(Q, X) {
    let Y = await this.safeParseAsync(Q, X);
    if (Y.success) return Y.data;
    throw Y.error;
  }
  async safeParseAsync(Q, X) {
    let Y = { common: { issues: [], contextualErrorMap: X?.errorMap, async: true }, path: X?.path || [], schemaErrorMap: this._def.errorMap, parent: null, data: Q, parsedType: M1(Q) }, $ = this._parse({ data: Q, path: Y.path, parent: Y }), J = await (t6($) ? $ : Promise.resolve($));
    return qJ(Y, J);
  }
  refine(Q, X) {
    let Y = ($) => {
      if (typeof X === "string" || typeof X > "u") return { message: X };
      else if (typeof X === "function") return X($);
      else return X;
    };
    return this._refinement(($, J) => {
      let W = Q($), G = () => J.addIssue({ code: w.custom, ...Y($) });
      if (typeof Promise < "u" && W instanceof Promise) return W.then((H) => {
        if (!H) return G(), false;
        else return true;
      });
      if (!W) return G(), false;
      else return true;
    });
  }
  refinement(Q, X) {
    return this._refinement((Y, $) => {
      if (!Q(Y)) return $.addIssue(typeof X === "function" ? X(Y, $) : X), false;
      else return true;
    });
  }
  _refinement(Q) {
    return new W1({ schema: this, typeName: j.ZodEffects, effect: { type: "refinement", refinement: Q } });
  }
  superRefine(Q) {
    return this._refinement(Q);
  }
  constructor(Q) {
    this.spa = this.safeParseAsync, this._def = Q, this.parse = this.parse.bind(this), this.safeParse = this.safeParse.bind(this), this.parseAsync = this.parseAsync.bind(this), this.safeParseAsync = this.safeParseAsync.bind(this), this.spa = this.spa.bind(this), this.refine = this.refine.bind(this), this.refinement = this.refinement.bind(this), this.superRefine = this.superRefine.bind(this), this.optional = this.optional.bind(this), this.nullable = this.nullable.bind(this), this.nullish = this.nullish.bind(this), this.array = this.array.bind(this), this.promise = this.promise.bind(this), this.or = this.or.bind(this), this.and = this.and.bind(this), this.transform = this.transform.bind(this), this.brand = this.brand.bind(this), this.default = this.default.bind(this), this.catch = this.catch.bind(this), this.describe = this.describe.bind(this), this.pipe = this.pipe.bind(this), this.readonly = this.readonly.bind(this), this.isNullable = this.isNullable.bind(this), this.isOptional = this.isOptional.bind(this), this["~standard"] = { version: 1, vendor: "zod", validate: (X) => this["~validate"](X) };
  }
  optional() {
    return m0.create(this, this._def);
  }
  nullable() {
    return S1.create(this, this._def);
  }
  nullish() {
    return this.nullable().optional();
  }
  array() {
    return $1.create(this);
  }
  promise() {
    return D6.create(this, this._def);
  }
  or(Q) {
    return Y9.create([this, Q], this._def);
  }
  and(Q) {
    return $9.create(this, Q, this._def);
  }
  transform(Q) {
    return new W1({ ...m(this._def), schema: this, typeName: j.ZodEffects, effect: { type: "transform", transform: Q } });
  }
  default(Q) {
    let X = typeof Q === "function" ? Q : () => Q;
    return new H9({ ...m(this._def), innerType: this, defaultValue: X, typeName: j.ZodDefault });
  }
  brand() {
    return new $8({ typeName: j.ZodBranded, type: this, ...m(this._def) });
  }
  catch(Q) {
    let X = typeof Q === "function" ? Q : () => Q;
    return new B9({ ...m(this._def), innerType: this, catchValue: X, typeName: j.ZodCatch });
  }
  describe(Q) {
    return new this.constructor({ ...this._def, description: Q });
  }
  pipe(Q) {
    return n9.create(this, Q);
  }
  readonly() {
    return z9.create(this);
  }
  isOptional() {
    return this.safeParse(void 0).success;
  }
  isNullable() {
    return this.safeParse(null).success;
  }
};
var HL = /^c[^\s-]{8,}$/i;
var BL = /^[0-9a-z]+$/;
var zL = /^[0-9A-HJKMNP-TV-Z]{26}$/i;
var KL = /^[0-9a-fA-F]{8}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{4}\b-[0-9a-fA-F]{12}$/i;
var VL = /^[a-z0-9_-]{21}$/i;
var qL = /^[A-Za-z0-9-_]+\.[A-Za-z0-9-_]+\.[A-Za-z0-9-_]*$/;
var UL = /^[-+]?P(?!$)(?:(?:[-+]?\d+Y)|(?:[-+]?\d+[.,]\d+Y$))?(?:(?:[-+]?\d+M)|(?:[-+]?\d+[.,]\d+M$))?(?:(?:[-+]?\d+W)|(?:[-+]?\d+[.,]\d+W$))?(?:(?:[-+]?\d+D)|(?:[-+]?\d+[.,]\d+D$))?(?:T(?=[\d+-])(?:(?:[-+]?\d+H)|(?:[-+]?\d+[.,]\d+H$))?(?:(?:[-+]?\d+M)|(?:[-+]?\d+[.,]\d+M$))?(?:[-+]?\d+(?:[.,]\d+)?S)?)??$/;
var LL = /^(?!\.)(?!.*\.\.)([A-Z0-9_'+\-\.]*)[A-Z0-9_+-]@([A-Z0-9][A-Z0-9\-]*\.)+[A-Z]{2,}$/i;
var FL = "^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$";
var QX;
var NL = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/;
var OL = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/(3[0-2]|[12]?[0-9])$/;
var DL = /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))$/;
var ML = /^(([0-9a-fA-F]{1,4}:){7,7}[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,7}:|([0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}|([0-9a-fA-F]{1,4}:){1,5}(:[0-9a-fA-F]{1,4}){1,2}|([0-9a-fA-F]{1,4}:){1,4}(:[0-9a-fA-F]{1,4}){1,3}|([0-9a-fA-F]{1,4}:){1,3}(:[0-9a-fA-F]{1,4}){1,4}|([0-9a-fA-F]{1,4}:){1,2}(:[0-9a-fA-F]{1,4}){1,5}|[0-9a-fA-F]{1,4}:((:[0-9a-fA-F]{1,4}){1,6})|:((:[0-9a-fA-F]{1,4}){1,7}|:)|fe80:(:[0-9a-fA-F]{0,4}){0,4}%[0-9a-zA-Z]{1,}|::(ffff(:0{1,4}){0,1}:){0,1}((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])|([0-9a-fA-F]{1,4}:){1,4}:((25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9])\.){3,3}(25[0-5]|(2[0-4]|1{0,1}[0-9]){0,1}[0-9]))\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/;
var wL = /^([0-9a-zA-Z+/]{4})*(([0-9a-zA-Z+/]{2}==)|([0-9a-zA-Z+/]{3}=))?$/;
var AL = /^([0-9a-zA-Z-_]{4})*(([0-9a-zA-Z-_]{2}(==)?)|([0-9a-zA-Z-_]{3}(=)?))?$/;
var LJ = "((\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-((0[13578]|1[02])-(0[1-9]|[12]\\d|3[01])|(0[469]|11)-(0[1-9]|[12]\\d|30)|(02)-(0[1-9]|1\\d|2[0-8])))";
var jL = new RegExp(`^${LJ}$`);
function FJ(Q) {
  let X = "[0-5]\\d";
  if (Q.precision) X = `${X}\\.\\d{${Q.precision}}`;
  else if (Q.precision == null) X = `${X}(\\.\\d+)?`;
  let Y = Q.precision ? "+" : "?";
  return `([01]\\d|2[0-3]):[0-5]\\d(:${X})${Y}`;
}
function RL(Q) {
  return new RegExp(`^${FJ(Q)}$`);
}
function NJ(Q) {
  let X = `${LJ}T${FJ(Q)}`, Y = [];
  if (Y.push(Q.local ? "Z?" : "Z"), Q.offset) Y.push("([+-]\\d{2}:?\\d{2})");
  return X = `${X}(${Y.join("|")})`, new RegExp(`^${X}$`);
}
function IL(Q, X) {
  if ((X === "v4" || !X) && NL.test(Q)) return true;
  if ((X === "v6" || !X) && DL.test(Q)) return true;
  return false;
}
function bL(Q, X) {
  if (!qL.test(Q)) return false;
  try {
    let [Y] = Q.split(".");
    if (!Y) return false;
    let $ = Y.replace(/-/g, "+").replace(/_/g, "/").padEnd(Y.length + (4 - Y.length % 4) % 4, "="), J = JSON.parse(atob($));
    if (typeof J !== "object" || J === null) return false;
    if ("typ" in J && J?.typ !== "JWT") return false;
    if (!J.alg) return false;
    if (X && J.alg !== X) return false;
    return true;
  } catch {
    return false;
  }
}
function EL(Q, X) {
  if ((X === "v4" || !X) && OL.test(Q)) return true;
  if ((X === "v6" || !X) && ML.test(Q)) return true;
  return false;
}
var Y1 = class _Y1 extends p {
  _parse(Q) {
    if (this._def.coerce) Q.data = String(Q.data);
    if (this._getType(Q) !== I.string) {
      let J = this._getOrReturnCtx(Q);
      return b(J, { code: w.invalid_type, expected: I.string, received: J.parsedType }), x;
    }
    let Y = new A0(), $ = void 0;
    for (let J of this._def.checks) if (J.kind === "min") {
      if (Q.data.length < J.value) $ = this._getOrReturnCtx(Q, $), b($, { code: w.too_small, minimum: J.value, type: "string", inclusive: true, exact: false, message: J.message }), Y.dirty();
    } else if (J.kind === "max") {
      if (Q.data.length > J.value) $ = this._getOrReturnCtx(Q, $), b($, { code: w.too_big, maximum: J.value, type: "string", inclusive: true, exact: false, message: J.message }), Y.dirty();
    } else if (J.kind === "length") {
      let W = Q.data.length > J.value, G = Q.data.length < J.value;
      if (W || G) {
        if ($ = this._getOrReturnCtx(Q, $), W) b($, { code: w.too_big, maximum: J.value, type: "string", inclusive: true, exact: true, message: J.message });
        else if (G) b($, { code: w.too_small, minimum: J.value, type: "string", inclusive: true, exact: true, message: J.message });
        Y.dirty();
      }
    } else if (J.kind === "email") {
      if (!LL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "email", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "emoji") {
      if (!QX) QX = new RegExp(FL, "u");
      if (!QX.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "emoji", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "uuid") {
      if (!KL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "uuid", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "nanoid") {
      if (!VL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "nanoid", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "cuid") {
      if (!HL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "cuid", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "cuid2") {
      if (!BL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "cuid2", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "ulid") {
      if (!zL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "ulid", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "url") try {
      new URL(Q.data);
    } catch {
      $ = this._getOrReturnCtx(Q, $), b($, { validation: "url", code: w.invalid_string, message: J.message }), Y.dirty();
    }
    else if (J.kind === "regex") {
      if (J.regex.lastIndex = 0, !J.regex.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "regex", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "trim") Q.data = Q.data.trim();
    else if (J.kind === "includes") {
      if (!Q.data.includes(J.value, J.position)) $ = this._getOrReturnCtx(Q, $), b($, { code: w.invalid_string, validation: { includes: J.value, position: J.position }, message: J.message }), Y.dirty();
    } else if (J.kind === "toLowerCase") Q.data = Q.data.toLowerCase();
    else if (J.kind === "toUpperCase") Q.data = Q.data.toUpperCase();
    else if (J.kind === "startsWith") {
      if (!Q.data.startsWith(J.value)) $ = this._getOrReturnCtx(Q, $), b($, { code: w.invalid_string, validation: { startsWith: J.value }, message: J.message }), Y.dirty();
    } else if (J.kind === "endsWith") {
      if (!Q.data.endsWith(J.value)) $ = this._getOrReturnCtx(Q, $), b($, { code: w.invalid_string, validation: { endsWith: J.value }, message: J.message }), Y.dirty();
    } else if (J.kind === "datetime") {
      if (!NJ(J).test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { code: w.invalid_string, validation: "datetime", message: J.message }), Y.dirty();
    } else if (J.kind === "date") {
      if (!jL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { code: w.invalid_string, validation: "date", message: J.message }), Y.dirty();
    } else if (J.kind === "time") {
      if (!RL(J).test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { code: w.invalid_string, validation: "time", message: J.message }), Y.dirty();
    } else if (J.kind === "duration") {
      if (!UL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "duration", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "ip") {
      if (!IL(Q.data, J.version)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "ip", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "jwt") {
      if (!bL(Q.data, J.alg)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "jwt", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "cidr") {
      if (!EL(Q.data, J.version)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "cidr", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "base64") {
      if (!wL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "base64", code: w.invalid_string, message: J.message }), Y.dirty();
    } else if (J.kind === "base64url") {
      if (!AL.test(Q.data)) $ = this._getOrReturnCtx(Q, $), b($, { validation: "base64url", code: w.invalid_string, message: J.message }), Y.dirty();
    } else d.assertNever(J);
    return { status: Y.value, value: Q.data };
  }
  _regex(Q, X, Y) {
    return this.refinement(($) => Q.test($), { validation: X, code: w.invalid_string, ...S.errToObj(Y) });
  }
  _addCheck(Q) {
    return new _Y1({ ...this._def, checks: [...this._def.checks, Q] });
  }
  email(Q) {
    return this._addCheck({ kind: "email", ...S.errToObj(Q) });
  }
  url(Q) {
    return this._addCheck({ kind: "url", ...S.errToObj(Q) });
  }
  emoji(Q) {
    return this._addCheck({ kind: "emoji", ...S.errToObj(Q) });
  }
  uuid(Q) {
    return this._addCheck({ kind: "uuid", ...S.errToObj(Q) });
  }
  nanoid(Q) {
    return this._addCheck({ kind: "nanoid", ...S.errToObj(Q) });
  }
  cuid(Q) {
    return this._addCheck({ kind: "cuid", ...S.errToObj(Q) });
  }
  cuid2(Q) {
    return this._addCheck({ kind: "cuid2", ...S.errToObj(Q) });
  }
  ulid(Q) {
    return this._addCheck({ kind: "ulid", ...S.errToObj(Q) });
  }
  base64(Q) {
    return this._addCheck({ kind: "base64", ...S.errToObj(Q) });
  }
  base64url(Q) {
    return this._addCheck({ kind: "base64url", ...S.errToObj(Q) });
  }
  jwt(Q) {
    return this._addCheck({ kind: "jwt", ...S.errToObj(Q) });
  }
  ip(Q) {
    return this._addCheck({ kind: "ip", ...S.errToObj(Q) });
  }
  cidr(Q) {
    return this._addCheck({ kind: "cidr", ...S.errToObj(Q) });
  }
  datetime(Q) {
    if (typeof Q === "string") return this._addCheck({ kind: "datetime", precision: null, offset: false, local: false, message: Q });
    return this._addCheck({ kind: "datetime", precision: typeof Q?.precision > "u" ? null : Q?.precision, offset: Q?.offset ?? false, local: Q?.local ?? false, ...S.errToObj(Q?.message) });
  }
  date(Q) {
    return this._addCheck({ kind: "date", message: Q });
  }
  time(Q) {
    if (typeof Q === "string") return this._addCheck({ kind: "time", precision: null, message: Q });
    return this._addCheck({ kind: "time", precision: typeof Q?.precision > "u" ? null : Q?.precision, ...S.errToObj(Q?.message) });
  }
  duration(Q) {
    return this._addCheck({ kind: "duration", ...S.errToObj(Q) });
  }
  regex(Q, X) {
    return this._addCheck({ kind: "regex", regex: Q, ...S.errToObj(X) });
  }
  includes(Q, X) {
    return this._addCheck({ kind: "includes", value: Q, position: X?.position, ...S.errToObj(X?.message) });
  }
  startsWith(Q, X) {
    return this._addCheck({ kind: "startsWith", value: Q, ...S.errToObj(X) });
  }
  endsWith(Q, X) {
    return this._addCheck({ kind: "endsWith", value: Q, ...S.errToObj(X) });
  }
  min(Q, X) {
    return this._addCheck({ kind: "min", value: Q, ...S.errToObj(X) });
  }
  max(Q, X) {
    return this._addCheck({ kind: "max", value: Q, ...S.errToObj(X) });
  }
  length(Q, X) {
    return this._addCheck({ kind: "length", value: Q, ...S.errToObj(X) });
  }
  nonempty(Q) {
    return this.min(1, S.errToObj(Q));
  }
  trim() {
    return new _Y1({ ...this._def, checks: [...this._def.checks, { kind: "trim" }] });
  }
  toLowerCase() {
    return new _Y1({ ...this._def, checks: [...this._def.checks, { kind: "toLowerCase" }] });
  }
  toUpperCase() {
    return new _Y1({ ...this._def, checks: [...this._def.checks, { kind: "toUpperCase" }] });
  }
  get isDatetime() {
    return !!this._def.checks.find((Q) => Q.kind === "datetime");
  }
  get isDate() {
    return !!this._def.checks.find((Q) => Q.kind === "date");
  }
  get isTime() {
    return !!this._def.checks.find((Q) => Q.kind === "time");
  }
  get isDuration() {
    return !!this._def.checks.find((Q) => Q.kind === "duration");
  }
  get isEmail() {
    return !!this._def.checks.find((Q) => Q.kind === "email");
  }
  get isURL() {
    return !!this._def.checks.find((Q) => Q.kind === "url");
  }
  get isEmoji() {
    return !!this._def.checks.find((Q) => Q.kind === "emoji");
  }
  get isUUID() {
    return !!this._def.checks.find((Q) => Q.kind === "uuid");
  }
  get isNANOID() {
    return !!this._def.checks.find((Q) => Q.kind === "nanoid");
  }
  get isCUID() {
    return !!this._def.checks.find((Q) => Q.kind === "cuid");
  }
  get isCUID2() {
    return !!this._def.checks.find((Q) => Q.kind === "cuid2");
  }
  get isULID() {
    return !!this._def.checks.find((Q) => Q.kind === "ulid");
  }
  get isIP() {
    return !!this._def.checks.find((Q) => Q.kind === "ip");
  }
  get isCIDR() {
    return !!this._def.checks.find((Q) => Q.kind === "cidr");
  }
  get isBase64() {
    return !!this._def.checks.find((Q) => Q.kind === "base64");
  }
  get isBase64url() {
    return !!this._def.checks.find((Q) => Q.kind === "base64url");
  }
  get minLength() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "min") {
      if (Q === null || X.value > Q) Q = X.value;
    }
    return Q;
  }
  get maxLength() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "max") {
      if (Q === null || X.value < Q) Q = X.value;
    }
    return Q;
  }
};
Y1.create = (Q) => {
  return new Y1({ checks: [], typeName: j.ZodString, coerce: Q?.coerce ?? false, ...m(Q) });
};
function PL(Q, X) {
  let Y = (Q.toString().split(".")[1] || "").length, $ = (X.toString().split(".")[1] || "").length, J = Y > $ ? Y : $, W = Number.parseInt(Q.toFixed(J).replace(".", "")), G = Number.parseInt(X.toFixed(J).replace(".", ""));
  return W % G / 10 ** J;
}
var c1 = class _c1 extends p {
  constructor() {
    super(...arguments);
    this.min = this.gte, this.max = this.lte, this.step = this.multipleOf;
  }
  _parse(Q) {
    if (this._def.coerce) Q.data = Number(Q.data);
    if (this._getType(Q) !== I.number) {
      let J = this._getOrReturnCtx(Q);
      return b(J, { code: w.invalid_type, expected: I.number, received: J.parsedType }), x;
    }
    let Y = void 0, $ = new A0();
    for (let J of this._def.checks) if (J.kind === "int") {
      if (!d.isInteger(Q.data)) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.invalid_type, expected: "integer", received: "float", message: J.message }), $.dirty();
    } else if (J.kind === "min") {
      if (J.inclusive ? Q.data < J.value : Q.data <= J.value) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.too_small, minimum: J.value, type: "number", inclusive: J.inclusive, exact: false, message: J.message }), $.dirty();
    } else if (J.kind === "max") {
      if (J.inclusive ? Q.data > J.value : Q.data >= J.value) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.too_big, maximum: J.value, type: "number", inclusive: J.inclusive, exact: false, message: J.message }), $.dirty();
    } else if (J.kind === "multipleOf") {
      if (PL(Q.data, J.value) !== 0) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.not_multiple_of, multipleOf: J.value, message: J.message }), $.dirty();
    } else if (J.kind === "finite") {
      if (!Number.isFinite(Q.data)) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.not_finite, message: J.message }), $.dirty();
    } else d.assertNever(J);
    return { status: $.value, value: Q.data };
  }
  gte(Q, X) {
    return this.setLimit("min", Q, true, S.toString(X));
  }
  gt(Q, X) {
    return this.setLimit("min", Q, false, S.toString(X));
  }
  lte(Q, X) {
    return this.setLimit("max", Q, true, S.toString(X));
  }
  lt(Q, X) {
    return this.setLimit("max", Q, false, S.toString(X));
  }
  setLimit(Q, X, Y, $) {
    return new _c1({ ...this._def, checks: [...this._def.checks, { kind: Q, value: X, inclusive: Y, message: S.toString($) }] });
  }
  _addCheck(Q) {
    return new _c1({ ...this._def, checks: [...this._def.checks, Q] });
  }
  int(Q) {
    return this._addCheck({ kind: "int", message: S.toString(Q) });
  }
  positive(Q) {
    return this._addCheck({ kind: "min", value: 0, inclusive: false, message: S.toString(Q) });
  }
  negative(Q) {
    return this._addCheck({ kind: "max", value: 0, inclusive: false, message: S.toString(Q) });
  }
  nonpositive(Q) {
    return this._addCheck({ kind: "max", value: 0, inclusive: true, message: S.toString(Q) });
  }
  nonnegative(Q) {
    return this._addCheck({ kind: "min", value: 0, inclusive: true, message: S.toString(Q) });
  }
  multipleOf(Q, X) {
    return this._addCheck({ kind: "multipleOf", value: Q, message: S.toString(X) });
  }
  finite(Q) {
    return this._addCheck({ kind: "finite", message: S.toString(Q) });
  }
  safe(Q) {
    return this._addCheck({ kind: "min", inclusive: true, value: Number.MIN_SAFE_INTEGER, message: S.toString(Q) })._addCheck({ kind: "max", inclusive: true, value: Number.MAX_SAFE_INTEGER, message: S.toString(Q) });
  }
  get minValue() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "min") {
      if (Q === null || X.value > Q) Q = X.value;
    }
    return Q;
  }
  get maxValue() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "max") {
      if (Q === null || X.value < Q) Q = X.value;
    }
    return Q;
  }
  get isInt() {
    return !!this._def.checks.find((Q) => Q.kind === "int" || Q.kind === "multipleOf" && d.isInteger(Q.value));
  }
  get isFinite() {
    let Q = null, X = null;
    for (let Y of this._def.checks) if (Y.kind === "finite" || Y.kind === "int" || Y.kind === "multipleOf") return true;
    else if (Y.kind === "min") {
      if (X === null || Y.value > X) X = Y.value;
    } else if (Y.kind === "max") {
      if (Q === null || Y.value < Q) Q = Y.value;
    }
    return Number.isFinite(X) && Number.isFinite(Q);
  }
};
c1.create = (Q) => {
  return new c1({ checks: [], typeName: j.ZodNumber, coerce: Q?.coerce || false, ...m(Q) });
};
var p1 = class _p1 extends p {
  constructor() {
    super(...arguments);
    this.min = this.gte, this.max = this.lte;
  }
  _parse(Q) {
    if (this._def.coerce) try {
      Q.data = BigInt(Q.data);
    } catch {
      return this._getInvalidInput(Q);
    }
    if (this._getType(Q) !== I.bigint) return this._getInvalidInput(Q);
    let Y = void 0, $ = new A0();
    for (let J of this._def.checks) if (J.kind === "min") {
      if (J.inclusive ? Q.data < J.value : Q.data <= J.value) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.too_small, type: "bigint", minimum: J.value, inclusive: J.inclusive, message: J.message }), $.dirty();
    } else if (J.kind === "max") {
      if (J.inclusive ? Q.data > J.value : Q.data >= J.value) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.too_big, type: "bigint", maximum: J.value, inclusive: J.inclusive, message: J.message }), $.dirty();
    } else if (J.kind === "multipleOf") {
      if (Q.data % J.value !== BigInt(0)) Y = this._getOrReturnCtx(Q, Y), b(Y, { code: w.not_multiple_of, multipleOf: J.value, message: J.message }), $.dirty();
    } else d.assertNever(J);
    return { status: $.value, value: Q.data };
  }
  _getInvalidInput(Q) {
    let X = this._getOrReturnCtx(Q);
    return b(X, { code: w.invalid_type, expected: I.bigint, received: X.parsedType }), x;
  }
  gte(Q, X) {
    return this.setLimit("min", Q, true, S.toString(X));
  }
  gt(Q, X) {
    return this.setLimit("min", Q, false, S.toString(X));
  }
  lte(Q, X) {
    return this.setLimit("max", Q, true, S.toString(X));
  }
  lt(Q, X) {
    return this.setLimit("max", Q, false, S.toString(X));
  }
  setLimit(Q, X, Y, $) {
    return new _p1({ ...this._def, checks: [...this._def.checks, { kind: Q, value: X, inclusive: Y, message: S.toString($) }] });
  }
  _addCheck(Q) {
    return new _p1({ ...this._def, checks: [...this._def.checks, Q] });
  }
  positive(Q) {
    return this._addCheck({ kind: "min", value: BigInt(0), inclusive: false, message: S.toString(Q) });
  }
  negative(Q) {
    return this._addCheck({ kind: "max", value: BigInt(0), inclusive: false, message: S.toString(Q) });
  }
  nonpositive(Q) {
    return this._addCheck({ kind: "max", value: BigInt(0), inclusive: true, message: S.toString(Q) });
  }
  nonnegative(Q) {
    return this._addCheck({ kind: "min", value: BigInt(0), inclusive: true, message: S.toString(Q) });
  }
  multipleOf(Q, X) {
    return this._addCheck({ kind: "multipleOf", value: Q, message: S.toString(X) });
  }
  get minValue() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "min") {
      if (Q === null || X.value > Q) Q = X.value;
    }
    return Q;
  }
  get maxValue() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "max") {
      if (Q === null || X.value < Q) Q = X.value;
    }
    return Q;
  }
};
p1.create = (Q) => {
  return new p1({ checks: [], typeName: j.ZodBigInt, coerce: Q?.coerce ?? false, ...m(Q) });
};
var e6 = class extends p {
  _parse(Q) {
    if (this._def.coerce) Q.data = Boolean(Q.data);
    if (this._getType(Q) !== I.boolean) {
      let Y = this._getOrReturnCtx(Q);
      return b(Y, { code: w.invalid_type, expected: I.boolean, received: Y.parsedType }), x;
    }
    return P0(Q.data);
  }
};
e6.create = (Q) => {
  return new e6({ typeName: j.ZodBoolean, coerce: Q?.coerce || false, ...m(Q) });
};
var F6 = class _F6 extends p {
  _parse(Q) {
    if (this._def.coerce) Q.data = new Date(Q.data);
    if (this._getType(Q) !== I.date) {
      let J = this._getOrReturnCtx(Q);
      return b(J, { code: w.invalid_type, expected: I.date, received: J.parsedType }), x;
    }
    if (Number.isNaN(Q.data.getTime())) {
      let J = this._getOrReturnCtx(Q);
      return b(J, { code: w.invalid_date }), x;
    }
    let Y = new A0(), $ = void 0;
    for (let J of this._def.checks) if (J.kind === "min") {
      if (Q.data.getTime() < J.value) $ = this._getOrReturnCtx(Q, $), b($, { code: w.too_small, message: J.message, inclusive: true, exact: false, minimum: J.value, type: "date" }), Y.dirty();
    } else if (J.kind === "max") {
      if (Q.data.getTime() > J.value) $ = this._getOrReturnCtx(Q, $), b($, { code: w.too_big, message: J.message, inclusive: true, exact: false, maximum: J.value, type: "date" }), Y.dirty();
    } else d.assertNever(J);
    return { status: Y.value, value: new Date(Q.data.getTime()) };
  }
  _addCheck(Q) {
    return new _F6({ ...this._def, checks: [...this._def.checks, Q] });
  }
  min(Q, X) {
    return this._addCheck({ kind: "min", value: Q.getTime(), message: S.toString(X) });
  }
  max(Q, X) {
    return this._addCheck({ kind: "max", value: Q.getTime(), message: S.toString(X) });
  }
  get minDate() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "min") {
      if (Q === null || X.value > Q) Q = X.value;
    }
    return Q != null ? new Date(Q) : null;
  }
  get maxDate() {
    let Q = null;
    for (let X of this._def.checks) if (X.kind === "max") {
      if (Q === null || X.value < Q) Q = X.value;
    }
    return Q != null ? new Date(Q) : null;
  }
};
F6.create = (Q) => {
  return new F6({ checks: [], coerce: Q?.coerce || false, typeName: j.ZodDate, ...m(Q) });
};
var l9 = class extends p {
  _parse(Q) {
    if (this._getType(Q) !== I.symbol) {
      let Y = this._getOrReturnCtx(Q);
      return b(Y, { code: w.invalid_type, expected: I.symbol, received: Y.parsedType }), x;
    }
    return P0(Q.data);
  }
};
l9.create = (Q) => {
  return new l9({ typeName: j.ZodSymbol, ...m(Q) });
};
var Q9 = class extends p {
  _parse(Q) {
    if (this._getType(Q) !== I.undefined) {
      let Y = this._getOrReturnCtx(Q);
      return b(Y, { code: w.invalid_type, expected: I.undefined, received: Y.parsedType }), x;
    }
    return P0(Q.data);
  }
};
Q9.create = (Q) => {
  return new Q9({ typeName: j.ZodUndefined, ...m(Q) });
};
var X9 = class extends p {
  _parse(Q) {
    if (this._getType(Q) !== I.null) {
      let Y = this._getOrReturnCtx(Q);
      return b(Y, { code: w.invalid_type, expected: I.null, received: Y.parsedType }), x;
    }
    return P0(Q.data);
  }
};
X9.create = (Q) => {
  return new X9({ typeName: j.ZodNull, ...m(Q) });
};
var N6 = class extends p {
  constructor() {
    super(...arguments);
    this._any = true;
  }
  _parse(Q) {
    return P0(Q.data);
  }
};
N6.create = (Q) => {
  return new N6({ typeName: j.ZodAny, ...m(Q) });
};
var l1 = class extends p {
  constructor() {
    super(...arguments);
    this._unknown = true;
  }
  _parse(Q) {
    return P0(Q.data);
  }
};
l1.create = (Q) => {
  return new l1({ typeName: j.ZodUnknown, ...m(Q) });
};
var w1 = class extends p {
  _parse(Q) {
    let X = this._getOrReturnCtx(Q);
    return b(X, { code: w.invalid_type, expected: I.never, received: X.parsedType }), x;
  }
};
w1.create = (Q) => {
  return new w1({ typeName: j.ZodNever, ...m(Q) });
};
var c9 = class extends p {
  _parse(Q) {
    if (this._getType(Q) !== I.undefined) {
      let Y = this._getOrReturnCtx(Q);
      return b(Y, { code: w.invalid_type, expected: I.void, received: Y.parsedType }), x;
    }
    return P0(Q.data);
  }
};
c9.create = (Q) => {
  return new c9({ typeName: j.ZodVoid, ...m(Q) });
};
var $1 = class _$1 extends p {
  _parse(Q) {
    let { ctx: X, status: Y } = this._processInputParams(Q), $ = this._def;
    if (X.parsedType !== I.array) return b(X, { code: w.invalid_type, expected: I.array, received: X.parsedType }), x;
    if ($.exactLength !== null) {
      let W = X.data.length > $.exactLength.value, G = X.data.length < $.exactLength.value;
      if (W || G) b(X, { code: W ? w.too_big : w.too_small, minimum: G ? $.exactLength.value : void 0, maximum: W ? $.exactLength.value : void 0, type: "array", inclusive: true, exact: true, message: $.exactLength.message }), Y.dirty();
    }
    if ($.minLength !== null) {
      if (X.data.length < $.minLength.value) b(X, { code: w.too_small, minimum: $.minLength.value, type: "array", inclusive: true, exact: false, message: $.minLength.message }), Y.dirty();
    }
    if ($.maxLength !== null) {
      if (X.data.length > $.maxLength.value) b(X, { code: w.too_big, maximum: $.maxLength.value, type: "array", inclusive: true, exact: false, message: $.maxLength.message }), Y.dirty();
    }
    if (X.common.async) return Promise.all([...X.data].map((W, G) => {
      return $.type._parseAsync(new J1(X, W, X.path, G));
    })).then((W) => {
      return A0.mergeArray(Y, W);
    });
    let J = [...X.data].map((W, G) => {
      return $.type._parseSync(new J1(X, W, X.path, G));
    });
    return A0.mergeArray(Y, J);
  }
  get element() {
    return this._def.type;
  }
  min(Q, X) {
    return new _$1({ ...this._def, minLength: { value: Q, message: S.toString(X) } });
  }
  max(Q, X) {
    return new _$1({ ...this._def, maxLength: { value: Q, message: S.toString(X) } });
  }
  length(Q, X) {
    return new _$1({ ...this._def, exactLength: { value: Q, message: S.toString(X) } });
  }
  nonempty(Q) {
    return this.min(1, Q);
  }
};
$1.create = (Q, X) => {
  return new $1({ type: Q, minLength: null, maxLength: null, exactLength: null, typeName: j.ZodArray, ...m(X) });
};
function a6(Q) {
  if (Q instanceof L0) {
    let X = {};
    for (let Y in Q.shape) {
      let $ = Q.shape[Y];
      X[Y] = m0.create(a6($));
    }
    return new L0({ ...Q._def, shape: () => X });
  } else if (Q instanceof $1) return new $1({ ...Q._def, type: a6(Q.element) });
  else if (Q instanceof m0) return m0.create(a6(Q.unwrap()));
  else if (Q instanceof S1) return S1.create(a6(Q.unwrap()));
  else if (Q instanceof A1) return A1.create(Q.items.map((X) => a6(X)));
  else return Q;
}
var L0 = class _L0 extends p {
  constructor() {
    super(...arguments);
    this._cached = null, this.nonstrict = this.passthrough, this.augment = this.extend;
  }
  _getCached() {
    if (this._cached !== null) return this._cached;
    let Q = this._def.shape(), X = d.objectKeys(Q);
    return this._cached = { shape: Q, keys: X }, this._cached;
  }
  _parse(Q) {
    if (this._getType(Q) !== I.object) {
      let B = this._getOrReturnCtx(Q);
      return b(B, { code: w.invalid_type, expected: I.object, received: B.parsedType }), x;
    }
    let { status: Y, ctx: $ } = this._processInputParams(Q), { shape: J, keys: W } = this._getCached(), G = [];
    if (!(this._def.catchall instanceof w1 && this._def.unknownKeys === "strip")) {
      for (let B in $.data) if (!W.includes(B)) G.push(B);
    }
    let H = [];
    for (let B of W) {
      let z = J[B], K = $.data[B];
      H.push({ key: { status: "valid", value: B }, value: z._parse(new J1($, K, $.path, B)), alwaysSet: B in $.data });
    }
    if (this._def.catchall instanceof w1) {
      let B = this._def.unknownKeys;
      if (B === "passthrough") for (let z of G) H.push({ key: { status: "valid", value: z }, value: { status: "valid", value: $.data[z] } });
      else if (B === "strict") {
        if (G.length > 0) b($, { code: w.unrecognized_keys, keys: G }), Y.dirty();
      } else if (B === "strip") ;
      else throw Error("Internal ZodObject error: invalid unknownKeys value.");
    } else {
      let B = this._def.catchall;
      for (let z of G) {
        let K = $.data[z];
        H.push({ key: { status: "valid", value: z }, value: B._parse(new J1($, K, $.path, z)), alwaysSet: z in $.data });
      }
    }
    if ($.common.async) return Promise.resolve().then(async () => {
      let B = [];
      for (let z of H) {
        let K = await z.key, U = await z.value;
        B.push({ key: K, value: U, alwaysSet: z.alwaysSet });
      }
      return B;
    }).then((B) => {
      return A0.mergeObjectSync(Y, B);
    });
    else return A0.mergeObjectSync(Y, H);
  }
  get shape() {
    return this._def.shape();
  }
  strict(Q) {
    return S.errToObj, new _L0({ ...this._def, unknownKeys: "strict", ...Q !== void 0 ? { errorMap: (X, Y) => {
      let $ = this._def.errorMap?.(X, Y).message ?? Y.defaultError;
      if (X.code === "unrecognized_keys") return { message: S.errToObj(Q).message ?? $ };
      return { message: $ };
    } } : {} });
  }
  strip() {
    return new _L0({ ...this._def, unknownKeys: "strip" });
  }
  passthrough() {
    return new _L0({ ...this._def, unknownKeys: "passthrough" });
  }
  extend(Q) {
    return new _L0({ ...this._def, shape: () => ({ ...this._def.shape(), ...Q }) });
  }
  merge(Q) {
    return new _L0({ unknownKeys: Q._def.unknownKeys, catchall: Q._def.catchall, shape: () => ({ ...this._def.shape(), ...Q._def.shape() }), typeName: j.ZodObject });
  }
  setKey(Q, X) {
    return this.augment({ [Q]: X });
  }
  catchall(Q) {
    return new _L0({ ...this._def, catchall: Q });
  }
  pick(Q) {
    let X = {};
    for (let Y of d.objectKeys(Q)) if (Q[Y] && this.shape[Y]) X[Y] = this.shape[Y];
    return new _L0({ ...this._def, shape: () => X });
  }
  omit(Q) {
    let X = {};
    for (let Y of d.objectKeys(this.shape)) if (!Q[Y]) X[Y] = this.shape[Y];
    return new _L0({ ...this._def, shape: () => X });
  }
  deepPartial() {
    return a6(this);
  }
  partial(Q) {
    let X = {};
    for (let Y of d.objectKeys(this.shape)) {
      let $ = this.shape[Y];
      if (Q && !Q[Y]) X[Y] = $;
      else X[Y] = $.optional();
    }
    return new _L0({ ...this._def, shape: () => X });
  }
  required(Q) {
    let X = {};
    for (let Y of d.objectKeys(this.shape)) if (Q && !Q[Y]) X[Y] = this.shape[Y];
    else {
      let J = this.shape[Y];
      while (J instanceof m0) J = J._def.innerType;
      X[Y] = J;
    }
    return new _L0({ ...this._def, shape: () => X });
  }
  keyof() {
    return OJ(d.objectKeys(this.shape));
  }
};
L0.create = (Q, X) => {
  return new L0({ shape: () => Q, unknownKeys: "strip", catchall: w1.create(), typeName: j.ZodObject, ...m(X) });
};
L0.strictCreate = (Q, X) => {
  return new L0({ shape: () => Q, unknownKeys: "strict", catchall: w1.create(), typeName: j.ZodObject, ...m(X) });
};
L0.lazycreate = (Q, X) => {
  return new L0({ shape: Q, unknownKeys: "strip", catchall: w1.create(), typeName: j.ZodObject, ...m(X) });
};
var Y9 = class extends p {
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q), Y = this._def.options;
    function $(J) {
      for (let G of J) if (G.result.status === "valid") return G.result;
      for (let G of J) if (G.result.status === "dirty") return X.common.issues.push(...G.ctx.common.issues), G.result;
      let W = J.map((G) => new x0(G.ctx.common.issues));
      return b(X, { code: w.invalid_union, unionErrors: W }), x;
    }
    if (X.common.async) return Promise.all(Y.map(async (J) => {
      let W = { ...X, common: { ...X.common, issues: [] }, parent: null };
      return { result: await J._parseAsync({ data: X.data, path: X.path, parent: W }), ctx: W };
    })).then($);
    else {
      let J = void 0, W = [];
      for (let H of Y) {
        let B = { ...X, common: { ...X.common, issues: [] }, parent: null }, z = H._parseSync({ data: X.data, path: X.path, parent: B });
        if (z.status === "valid") return z;
        else if (z.status === "dirty" && !J) J = { result: z, ctx: B };
        if (B.common.issues.length) W.push(B.common.issues);
      }
      if (J) return X.common.issues.push(...J.ctx.common.issues), J.result;
      let G = W.map((H) => new x0(H));
      return b(X, { code: w.invalid_union, unionErrors: G }), x;
    }
  }
  get options() {
    return this._def.options;
  }
};
Y9.create = (Q, X) => {
  return new Y9({ options: Q, typeName: j.ZodUnion, ...m(X) });
};
var C1 = (Q) => {
  if (Q instanceof J9) return C1(Q.schema);
  else if (Q instanceof W1) return C1(Q.innerType());
  else if (Q instanceof W9) return [Q.value];
  else if (Q instanceof d1) return Q.options;
  else if (Q instanceof G9) return d.objectValues(Q.enum);
  else if (Q instanceof H9) return C1(Q._def.innerType);
  else if (Q instanceof Q9) return [void 0];
  else if (Q instanceof X9) return [null];
  else if (Q instanceof m0) return [void 0, ...C1(Q.unwrap())];
  else if (Q instanceof S1) return [null, ...C1(Q.unwrap())];
  else if (Q instanceof $8) return C1(Q.unwrap());
  else if (Q instanceof z9) return C1(Q.unwrap());
  else if (Q instanceof B9) return C1(Q._def.innerType);
  else return [];
};
var Y8 = class _Y8 extends p {
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q);
    if (X.parsedType !== I.object) return b(X, { code: w.invalid_type, expected: I.object, received: X.parsedType }), x;
    let Y = this.discriminator, $ = X.data[Y], J = this.optionsMap.get($);
    if (!J) return b(X, { code: w.invalid_union_discriminator, options: Array.from(this.optionsMap.keys()), path: [Y] }), x;
    if (X.common.async) return J._parseAsync({ data: X.data, path: X.path, parent: X });
    else return J._parseSync({ data: X.data, path: X.path, parent: X });
  }
  get discriminator() {
    return this._def.discriminator;
  }
  get options() {
    return this._def.options;
  }
  get optionsMap() {
    return this._def.optionsMap;
  }
  static create(Q, X, Y) {
    let $ = /* @__PURE__ */ new Map();
    for (let J of X) {
      let W = C1(J.shape[Q]);
      if (!W.length) throw Error(`A discriminator value for key \`${Q}\` could not be extracted from all schema options`);
      for (let G of W) {
        if ($.has(G)) throw Error(`Discriminator property ${String(Q)} has duplicate value ${String(G)}`);
        $.set(G, J);
      }
    }
    return new _Y8({ typeName: j.ZodDiscriminatedUnion, discriminator: Q, options: X, optionsMap: $, ...m(Y) });
  }
};
function XX(Q, X) {
  let Y = M1(Q), $ = M1(X);
  if (Q === X) return { valid: true, data: Q };
  else if (Y === I.object && $ === I.object) {
    let J = d.objectKeys(X), W = d.objectKeys(Q).filter((H) => J.indexOf(H) !== -1), G = { ...Q, ...X };
    for (let H of W) {
      let B = XX(Q[H], X[H]);
      if (!B.valid) return { valid: false };
      G[H] = B.data;
    }
    return { valid: true, data: G };
  } else if (Y === I.array && $ === I.array) {
    if (Q.length !== X.length) return { valid: false };
    let J = [];
    for (let W = 0; W < Q.length; W++) {
      let G = Q[W], H = X[W], B = XX(G, H);
      if (!B.valid) return { valid: false };
      J.push(B.data);
    }
    return { valid: true, data: J };
  } else if (Y === I.date && $ === I.date && +Q === +X) return { valid: true, data: Q };
  else return { valid: false };
}
var $9 = class extends p {
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q), $ = (J, W) => {
      if (Q8(J) || Q8(W)) return x;
      let G = XX(J.value, W.value);
      if (!G.valid) return b(Y, { code: w.invalid_intersection_types }), x;
      if (X8(J) || X8(W)) X.dirty();
      return { status: X.value, value: G.data };
    };
    if (Y.common.async) return Promise.all([this._def.left._parseAsync({ data: Y.data, path: Y.path, parent: Y }), this._def.right._parseAsync({ data: Y.data, path: Y.path, parent: Y })]).then(([J, W]) => $(J, W));
    else return $(this._def.left._parseSync({ data: Y.data, path: Y.path, parent: Y }), this._def.right._parseSync({ data: Y.data, path: Y.path, parent: Y }));
  }
};
$9.create = (Q, X, Y) => {
  return new $9({ left: Q, right: X, typeName: j.ZodIntersection, ...m(Y) });
};
var A1 = class _A1 extends p {
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q);
    if (Y.parsedType !== I.array) return b(Y, { code: w.invalid_type, expected: I.array, received: Y.parsedType }), x;
    if (Y.data.length < this._def.items.length) return b(Y, { code: w.too_small, minimum: this._def.items.length, inclusive: true, exact: false, type: "array" }), x;
    if (!this._def.rest && Y.data.length > this._def.items.length) b(Y, { code: w.too_big, maximum: this._def.items.length, inclusive: true, exact: false, type: "array" }), X.dirty();
    let J = [...Y.data].map((W, G) => {
      let H = this._def.items[G] || this._def.rest;
      if (!H) return null;
      return H._parse(new J1(Y, W, Y.path, G));
    }).filter((W) => !!W);
    if (Y.common.async) return Promise.all(J).then((W) => {
      return A0.mergeArray(X, W);
    });
    else return A0.mergeArray(X, J);
  }
  get items() {
    return this._def.items;
  }
  rest(Q) {
    return new _A1({ ...this._def, rest: Q });
  }
};
A1.create = (Q, X) => {
  if (!Array.isArray(Q)) throw Error("You must pass an array of schemas to z.tuple([ ... ])");
  return new A1({ items: Q, typeName: j.ZodTuple, rest: null, ...m(X) });
};
var p9 = class _p9 extends p {
  get keySchema() {
    return this._def.keyType;
  }
  get valueSchema() {
    return this._def.valueType;
  }
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q);
    if (Y.parsedType !== I.object) return b(Y, { code: w.invalid_type, expected: I.object, received: Y.parsedType }), x;
    let $ = [], J = this._def.keyType, W = this._def.valueType;
    for (let G in Y.data) $.push({ key: J._parse(new J1(Y, G, Y.path, G)), value: W._parse(new J1(Y, Y.data[G], Y.path, G)), alwaysSet: G in Y.data });
    if (Y.common.async) return A0.mergeObjectAsync(X, $);
    else return A0.mergeObjectSync(X, $);
  }
  get element() {
    return this._def.valueType;
  }
  static create(Q, X, Y) {
    if (X instanceof p) return new _p9({ keyType: Q, valueType: X, typeName: j.ZodRecord, ...m(Y) });
    return new _p9({ keyType: Y1.create(), valueType: Q, typeName: j.ZodRecord, ...m(X) });
  }
};
var d9 = class extends p {
  get keySchema() {
    return this._def.keyType;
  }
  get valueSchema() {
    return this._def.valueType;
  }
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q);
    if (Y.parsedType !== I.map) return b(Y, { code: w.invalid_type, expected: I.map, received: Y.parsedType }), x;
    let $ = this._def.keyType, J = this._def.valueType, W = [...Y.data.entries()].map(([G, H], B) => {
      return { key: $._parse(new J1(Y, G, Y.path, [B, "key"])), value: J._parse(new J1(Y, H, Y.path, [B, "value"])) };
    });
    if (Y.common.async) {
      let G = /* @__PURE__ */ new Map();
      return Promise.resolve().then(async () => {
        for (let H of W) {
          let B = await H.key, z = await H.value;
          if (B.status === "aborted" || z.status === "aborted") return x;
          if (B.status === "dirty" || z.status === "dirty") X.dirty();
          G.set(B.value, z.value);
        }
        return { status: X.value, value: G };
      });
    } else {
      let G = /* @__PURE__ */ new Map();
      for (let H of W) {
        let { key: B, value: z } = H;
        if (B.status === "aborted" || z.status === "aborted") return x;
        if (B.status === "dirty" || z.status === "dirty") X.dirty();
        G.set(B.value, z.value);
      }
      return { status: X.value, value: G };
    }
  }
};
d9.create = (Q, X, Y) => {
  return new d9({ valueType: X, keyType: Q, typeName: j.ZodMap, ...m(Y) });
};
var O6 = class _O6 extends p {
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q);
    if (Y.parsedType !== I.set) return b(Y, { code: w.invalid_type, expected: I.set, received: Y.parsedType }), x;
    let $ = this._def;
    if ($.minSize !== null) {
      if (Y.data.size < $.minSize.value) b(Y, { code: w.too_small, minimum: $.minSize.value, type: "set", inclusive: true, exact: false, message: $.minSize.message }), X.dirty();
    }
    if ($.maxSize !== null) {
      if (Y.data.size > $.maxSize.value) b(Y, { code: w.too_big, maximum: $.maxSize.value, type: "set", inclusive: true, exact: false, message: $.maxSize.message }), X.dirty();
    }
    let J = this._def.valueType;
    function W(H) {
      let B = /* @__PURE__ */ new Set();
      for (let z of H) {
        if (z.status === "aborted") return x;
        if (z.status === "dirty") X.dirty();
        B.add(z.value);
      }
      return { status: X.value, value: B };
    }
    let G = [...Y.data.values()].map((H, B) => J._parse(new J1(Y, H, Y.path, B)));
    if (Y.common.async) return Promise.all(G).then((H) => W(H));
    else return W(G);
  }
  min(Q, X) {
    return new _O6({ ...this._def, minSize: { value: Q, message: S.toString(X) } });
  }
  max(Q, X) {
    return new _O6({ ...this._def, maxSize: { value: Q, message: S.toString(X) } });
  }
  size(Q, X) {
    return this.min(Q, X).max(Q, X);
  }
  nonempty(Q) {
    return this.min(1, Q);
  }
};
O6.create = (Q, X) => {
  return new O6({ valueType: Q, minSize: null, maxSize: null, typeName: j.ZodSet, ...m(X) });
};
var s6 = class _s6 extends p {
  constructor() {
    super(...arguments);
    this.validate = this.implement;
  }
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q);
    if (X.parsedType !== I.function) return b(X, { code: w.invalid_type, expected: I.function, received: X.parsedType }), x;
    function Y(G, H) {
      return m9({ data: G, path: X.path, errorMaps: [X.common.contextualErrorMap, X.schemaErrorMap, r6(), Z1].filter((B) => !!B), issueData: { code: w.invalid_arguments, argumentsError: H } });
    }
    function $(G, H) {
      return m9({ data: G, path: X.path, errorMaps: [X.common.contextualErrorMap, X.schemaErrorMap, r6(), Z1].filter((B) => !!B), issueData: { code: w.invalid_return_type, returnTypeError: H } });
    }
    let J = { errorMap: X.common.contextualErrorMap }, W = X.data;
    if (this._def.returns instanceof D6) {
      let G = this;
      return P0(async function(...H) {
        let B = new x0([]), z = await G._def.args.parseAsync(H, J).catch((q) => {
          throw B.addIssue(Y(H, q)), B;
        }), K = await Reflect.apply(W, this, z);
        return await G._def.returns._def.type.parseAsync(K, J).catch((q) => {
          throw B.addIssue($(K, q)), B;
        });
      });
    } else {
      let G = this;
      return P0(function(...H) {
        let B = G._def.args.safeParse(H, J);
        if (!B.success) throw new x0([Y(H, B.error)]);
        let z = Reflect.apply(W, this, B.data), K = G._def.returns.safeParse(z, J);
        if (!K.success) throw new x0([$(z, K.error)]);
        return K.data;
      });
    }
  }
  parameters() {
    return this._def.args;
  }
  returnType() {
    return this._def.returns;
  }
  args(...Q) {
    return new _s6({ ...this._def, args: A1.create(Q).rest(l1.create()) });
  }
  returns(Q) {
    return new _s6({ ...this._def, returns: Q });
  }
  implement(Q) {
    return this.parse(Q);
  }
  strictImplement(Q) {
    return this.parse(Q);
  }
  static create(Q, X, Y) {
    return new _s6({ args: Q ? Q : A1.create([]).rest(l1.create()), returns: X || l1.create(), typeName: j.ZodFunction, ...m(Y) });
  }
};
var J9 = class extends p {
  get schema() {
    return this._def.getter();
  }
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q);
    return this._def.getter()._parse({ data: X.data, path: X.path, parent: X });
  }
};
J9.create = (Q, X) => {
  return new J9({ getter: Q, typeName: j.ZodLazy, ...m(X) });
};
var W9 = class extends p {
  _parse(Q) {
    if (Q.data !== this._def.value) {
      let X = this._getOrReturnCtx(Q);
      return b(X, { received: X.data, code: w.invalid_literal, expected: this._def.value }), x;
    }
    return { status: "valid", value: Q.data };
  }
  get value() {
    return this._def.value;
  }
};
W9.create = (Q, X) => {
  return new W9({ value: Q, typeName: j.ZodLiteral, ...m(X) });
};
function OJ(Q, X) {
  return new d1({ values: Q, typeName: j.ZodEnum, ...m(X) });
}
var d1 = class _d1 extends p {
  _parse(Q) {
    if (typeof Q.data !== "string") {
      let X = this._getOrReturnCtx(Q), Y = this._def.values;
      return b(X, { expected: d.joinValues(Y), received: X.parsedType, code: w.invalid_type }), x;
    }
    if (!this._cache) this._cache = new Set(this._def.values);
    if (!this._cache.has(Q.data)) {
      let X = this._getOrReturnCtx(Q), Y = this._def.values;
      return b(X, { received: X.data, code: w.invalid_enum_value, options: Y }), x;
    }
    return P0(Q.data);
  }
  get options() {
    return this._def.values;
  }
  get enum() {
    let Q = {};
    for (let X of this._def.values) Q[X] = X;
    return Q;
  }
  get Values() {
    let Q = {};
    for (let X of this._def.values) Q[X] = X;
    return Q;
  }
  get Enum() {
    let Q = {};
    for (let X of this._def.values) Q[X] = X;
    return Q;
  }
  extract(Q, X = this._def) {
    return _d1.create(Q, { ...this._def, ...X });
  }
  exclude(Q, X = this._def) {
    return _d1.create(this.options.filter((Y) => !Q.includes(Y)), { ...this._def, ...X });
  }
};
d1.create = OJ;
var G9 = class extends p {
  _parse(Q) {
    let X = d.getValidEnumValues(this._def.values), Y = this._getOrReturnCtx(Q);
    if (Y.parsedType !== I.string && Y.parsedType !== I.number) {
      let $ = d.objectValues(X);
      return b(Y, { expected: d.joinValues($), received: Y.parsedType, code: w.invalid_type }), x;
    }
    if (!this._cache) this._cache = new Set(d.getValidEnumValues(this._def.values));
    if (!this._cache.has(Q.data)) {
      let $ = d.objectValues(X);
      return b(Y, { received: Y.data, code: w.invalid_enum_value, options: $ }), x;
    }
    return P0(Q.data);
  }
  get enum() {
    return this._def.values;
  }
};
G9.create = (Q, X) => {
  return new G9({ values: Q, typeName: j.ZodNativeEnum, ...m(X) });
};
var D6 = class extends p {
  unwrap() {
    return this._def.type;
  }
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q);
    if (X.parsedType !== I.promise && X.common.async === false) return b(X, { code: w.invalid_type, expected: I.promise, received: X.parsedType }), x;
    let Y = X.parsedType === I.promise ? X.data : Promise.resolve(X.data);
    return P0(Y.then(($) => {
      return this._def.type.parseAsync($, { path: X.path, errorMap: X.common.contextualErrorMap });
    }));
  }
};
D6.create = (Q, X) => {
  return new D6({ type: Q, typeName: j.ZodPromise, ...m(X) });
};
var W1 = class extends p {
  innerType() {
    return this._def.schema;
  }
  sourceType() {
    return this._def.schema._def.typeName === j.ZodEffects ? this._def.schema.sourceType() : this._def.schema;
  }
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q), $ = this._def.effect || null, J = { addIssue: (W) => {
      if (b(Y, W), W.fatal) X.abort();
      else X.dirty();
    }, get path() {
      return Y.path;
    } };
    if (J.addIssue = J.addIssue.bind(J), $.type === "preprocess") {
      let W = $.transform(Y.data, J);
      if (Y.common.async) return Promise.resolve(W).then(async (G) => {
        if (X.value === "aborted") return x;
        let H = await this._def.schema._parseAsync({ data: G, path: Y.path, parent: Y });
        if (H.status === "aborted") return x;
        if (H.status === "dirty") return L6(H.value);
        if (X.value === "dirty") return L6(H.value);
        return H;
      });
      else {
        if (X.value === "aborted") return x;
        let G = this._def.schema._parseSync({ data: W, path: Y.path, parent: Y });
        if (G.status === "aborted") return x;
        if (G.status === "dirty") return L6(G.value);
        if (X.value === "dirty") return L6(G.value);
        return G;
      }
    }
    if ($.type === "refinement") {
      let W = (G) => {
        let H = $.refinement(G, J);
        if (Y.common.async) return Promise.resolve(H);
        if (H instanceof Promise) throw Error("Async refinement encountered during synchronous parse operation. Use .parseAsync instead.");
        return G;
      };
      if (Y.common.async === false) {
        let G = this._def.schema._parseSync({ data: Y.data, path: Y.path, parent: Y });
        if (G.status === "aborted") return x;
        if (G.status === "dirty") X.dirty();
        return W(G.value), { status: X.value, value: G.value };
      } else return this._def.schema._parseAsync({ data: Y.data, path: Y.path, parent: Y }).then((G) => {
        if (G.status === "aborted") return x;
        if (G.status === "dirty") X.dirty();
        return W(G.value).then(() => {
          return { status: X.value, value: G.value };
        });
      });
    }
    if ($.type === "transform") if (Y.common.async === false) {
      let W = this._def.schema._parseSync({ data: Y.data, path: Y.path, parent: Y });
      if (!m1(W)) return x;
      let G = $.transform(W.value, J);
      if (G instanceof Promise) throw Error("Asynchronous transform encountered during synchronous parse operation. Use .parseAsync instead.");
      return { status: X.value, value: G };
    } else return this._def.schema._parseAsync({ data: Y.data, path: Y.path, parent: Y }).then((W) => {
      if (!m1(W)) return x;
      return Promise.resolve($.transform(W.value, J)).then((G) => ({ status: X.value, value: G }));
    });
    d.assertNever($);
  }
};
W1.create = (Q, X, Y) => {
  return new W1({ schema: Q, typeName: j.ZodEffects, effect: X, ...m(Y) });
};
W1.createWithPreprocess = (Q, X, Y) => {
  return new W1({ schema: X, effect: { type: "preprocess", transform: Q }, typeName: j.ZodEffects, ...m(Y) });
};
var m0 = class extends p {
  _parse(Q) {
    if (this._getType(Q) === I.undefined) return P0(void 0);
    return this._def.innerType._parse(Q);
  }
  unwrap() {
    return this._def.innerType;
  }
};
m0.create = (Q, X) => {
  return new m0({ innerType: Q, typeName: j.ZodOptional, ...m(X) });
};
var S1 = class extends p {
  _parse(Q) {
    if (this._getType(Q) === I.null) return P0(null);
    return this._def.innerType._parse(Q);
  }
  unwrap() {
    return this._def.innerType;
  }
};
S1.create = (Q, X) => {
  return new S1({ innerType: Q, typeName: j.ZodNullable, ...m(X) });
};
var H9 = class extends p {
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q), Y = X.data;
    if (X.parsedType === I.undefined) Y = this._def.defaultValue();
    return this._def.innerType._parse({ data: Y, path: X.path, parent: X });
  }
  removeDefault() {
    return this._def.innerType;
  }
};
H9.create = (Q, X) => {
  return new H9({ innerType: Q, typeName: j.ZodDefault, defaultValue: typeof X.default === "function" ? X.default : () => X.default, ...m(X) });
};
var B9 = class extends p {
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q), Y = { ...X, common: { ...X.common, issues: [] } }, $ = this._def.innerType._parse({ data: Y.data, path: Y.path, parent: { ...Y } });
    if (t6($)) return $.then((J) => {
      return { status: "valid", value: J.status === "valid" ? J.value : this._def.catchValue({ get error() {
        return new x0(Y.common.issues);
      }, input: Y.data }) };
    });
    else return { status: "valid", value: $.status === "valid" ? $.value : this._def.catchValue({ get error() {
      return new x0(Y.common.issues);
    }, input: Y.data }) };
  }
  removeCatch() {
    return this._def.innerType;
  }
};
B9.create = (Q, X) => {
  return new B9({ innerType: Q, typeName: j.ZodCatch, catchValue: typeof X.catch === "function" ? X.catch : () => X.catch, ...m(X) });
};
var i9 = class extends p {
  _parse(Q) {
    if (this._getType(Q) !== I.nan) {
      let Y = this._getOrReturnCtx(Q);
      return b(Y, { code: w.invalid_type, expected: I.nan, received: Y.parsedType }), x;
    }
    return { status: "valid", value: Q.data };
  }
};
i9.create = (Q) => {
  return new i9({ typeName: j.ZodNaN, ...m(Q) });
};
var ZL = /* @__PURE__ */ Symbol("zod_brand");
var $8 = class extends p {
  _parse(Q) {
    let { ctx: X } = this._processInputParams(Q), Y = X.data;
    return this._def.type._parse({ data: Y, path: X.path, parent: X });
  }
  unwrap() {
    return this._def.type;
  }
};
var n9 = class _n9 extends p {
  _parse(Q) {
    let { status: X, ctx: Y } = this._processInputParams(Q);
    if (Y.common.async) return (async () => {
      let J = await this._def.in._parseAsync({ data: Y.data, path: Y.path, parent: Y });
      if (J.status === "aborted") return x;
      if (J.status === "dirty") return X.dirty(), L6(J.value);
      else return this._def.out._parseAsync({ data: J.value, path: Y.path, parent: Y });
    })();
    else {
      let $ = this._def.in._parseSync({ data: Y.data, path: Y.path, parent: Y });
      if ($.status === "aborted") return x;
      if ($.status === "dirty") return X.dirty(), { status: "dirty", value: $.value };
      else return this._def.out._parseSync({ data: $.value, path: Y.path, parent: Y });
    }
  }
  static create(Q, X) {
    return new _n9({ in: Q, out: X, typeName: j.ZodPipeline });
  }
};
var z9 = class extends p {
  _parse(Q) {
    let X = this._def.innerType._parse(Q), Y = ($) => {
      if (m1($)) $.value = Object.freeze($.value);
      return $;
    };
    return t6(X) ? X.then(($) => Y($)) : Y(X);
  }
  unwrap() {
    return this._def.innerType;
  }
};
z9.create = (Q, X) => {
  return new z9({ innerType: Q, typeName: j.ZodReadonly, ...m(X) });
};
function UJ(Q, X) {
  let Y = typeof Q === "function" ? Q(X) : typeof Q === "string" ? { message: Q } : Q;
  return typeof Y === "string" ? { message: Y } : Y;
}
function DJ(Q, X = {}, Y) {
  if (Q) return N6.create().superRefine(($, J) => {
    let W = Q($);
    if (W instanceof Promise) return W.then((G) => {
      if (!G) {
        let H = UJ(X, $), B = H.fatal ?? Y ?? true;
        J.addIssue({ code: "custom", ...H, fatal: B });
      }
    });
    if (!W) {
      let G = UJ(X, $), H = G.fatal ?? Y ?? true;
      J.addIssue({ code: "custom", ...G, fatal: H });
    }
    return;
  });
  return N6.create();
}
var CL = { object: L0.lazycreate };
var j;
(function(Q) {
  Q.ZodString = "ZodString", Q.ZodNumber = "ZodNumber", Q.ZodNaN = "ZodNaN", Q.ZodBigInt = "ZodBigInt", Q.ZodBoolean = "ZodBoolean", Q.ZodDate = "ZodDate", Q.ZodSymbol = "ZodSymbol", Q.ZodUndefined = "ZodUndefined", Q.ZodNull = "ZodNull", Q.ZodAny = "ZodAny", Q.ZodUnknown = "ZodUnknown", Q.ZodNever = "ZodNever", Q.ZodVoid = "ZodVoid", Q.ZodArray = "ZodArray", Q.ZodObject = "ZodObject", Q.ZodUnion = "ZodUnion", Q.ZodDiscriminatedUnion = "ZodDiscriminatedUnion", Q.ZodIntersection = "ZodIntersection", Q.ZodTuple = "ZodTuple", Q.ZodRecord = "ZodRecord", Q.ZodMap = "ZodMap", Q.ZodSet = "ZodSet", Q.ZodFunction = "ZodFunction", Q.ZodLazy = "ZodLazy", Q.ZodLiteral = "ZodLiteral", Q.ZodEnum = "ZodEnum", Q.ZodEffects = "ZodEffects", Q.ZodNativeEnum = "ZodNativeEnum", Q.ZodOptional = "ZodOptional", Q.ZodNullable = "ZodNullable", Q.ZodDefault = "ZodDefault", Q.ZodCatch = "ZodCatch", Q.ZodPromise = "ZodPromise", Q.ZodBranded = "ZodBranded", Q.ZodPipeline = "ZodPipeline", Q.ZodReadonly = "ZodReadonly";
})(j || (j = {}));
var SL = (Q, X = { message: `Input not instance of ${Q.name}` }) => DJ((Y) => Y instanceof Q, X);
var MJ = Y1.create;
var wJ = c1.create;
var _L = i9.create;
var kL = p1.create;
var AJ = e6.create;
var vL = F6.create;
var TL = l9.create;
var xL = Q9.create;
var yL = X9.create;
var gL = N6.create;
var hL = l1.create;
var fL = w1.create;
var uL = c9.create;
var mL = $1.create;
var YX = L0.create;
var lL = L0.strictCreate;
var cL = Y9.create;
var pL = Y8.create;
var dL = $9.create;
var iL = A1.create;
var nL = p9.create;
var oL = d9.create;
var rL = O6.create;
var tL = s6.create;
var aL = J9.create;
var sL = W9.create;
var eL = d1.create;
var QF = G9.create;
var XF = D6.create;
var YF = W1.create;
var $F = m0.create;
var JF = S1.create;
var WF = W1.createWithPreprocess;
var GF = n9.create;
var HF = () => MJ().optional();
var BF = () => wJ().optional();
var zF = () => AJ().optional();
var KF = { string: (Q) => Y1.create({ ...Q, coerce: true }), number: (Q) => c1.create({ ...Q, coerce: true }), boolean: (Q) => e6.create({ ...Q, coerce: true }), bigint: (Q) => p1.create({ ...Q, coerce: true }), date: (Q) => F6.create({ ...Q, coerce: true }) };
var VF = x;
var qF = Object.freeze({ status: "aborted" });
function D(Q, X, Y) {
  function $(H, B) {
    var z;
    Object.defineProperty(H, "_zod", { value: H._zod ?? {}, enumerable: false }), (z = H._zod).traits ?? (z.traits = /* @__PURE__ */ new Set()), H._zod.traits.add(Q), X(H, B);
    for (let K in G.prototype) if (!(K in H)) Object.defineProperty(H, K, { value: G.prototype[K].bind(H) });
    H._zod.constr = G, H._zod.def = B;
  }
  let J = Y?.Parent ?? Object;
  class W extends J {
  }
  Object.defineProperty(W, "name", { value: Q });
  function G(H) {
    var B;
    let z = Y?.Parent ? new W() : this;
    $(z, H), (B = z._zod).deferred ?? (B.deferred = []);
    for (let K of z._zod.deferred) K();
    return z;
  }
  return Object.defineProperty(G, "init", { value: $ }), Object.defineProperty(G, Symbol.hasInstance, { value: (H) => {
    if (Y?.Parent && H instanceof Y.Parent) return true;
    return H?._zod?.traits?.has(Q);
  } }), Object.defineProperty(G, "name", { value: Q }), G;
}
var n1 = class extends Error {
  constructor() {
    super("Encountered Promise during synchronous parse. Use .parseAsync() instead.");
  }
};
var J8 = {};
function l0(Q) {
  if (Q) Object.assign(J8, Q);
  return J8;
}
var i = {};
gQ(i, { unwrapMessage: () => o9, stringifyPrimitive: () => H8, required: () => SF, randomString: () => AF, propertyKeyTypes: () => BX, promiseAllObject: () => wF, primitiveTypes: () => jJ, prefixIssues: () => j1, pick: () => bF, partial: () => CF, optionalKeys: () => zX, omit: () => EF, numKeys: () => jF, nullish: () => a9, normalizeParams: () => y, merge: () => ZF, jsonStringifyReplacer: () => JX, joinValues: () => W8, issue: () => VX, isPlainObject: () => V9, isObject: () => K9, getSizableOrigin: () => IJ, getParsedType: () => RF, getLengthableOrigin: () => e9, getEnumValues: () => r9, getElementAtPath: () => MF, floatSafeRemainder: () => WX, finalizeIssue: () => G1, extend: () => PF, escapeRegex: () => o1, esc: () => M6, defineLazy: () => $0, createTransparentProxy: () => IF, clone: () => c0, cleanRegex: () => s9, cleanEnum: () => _F, captureStackTrace: () => G8, cached: () => t9, assignProp: () => GX, assertNotEqual: () => FF, assertNever: () => OF, assertIs: () => NF, assertEqual: () => LF, assert: () => DF, allowsEval: () => HX, aborted: () => w6, NUMBER_FORMAT_RANGES: () => KX, Class: () => bJ, BIGINT_FORMAT_RANGES: () => RJ });
function LF(Q) {
  return Q;
}
function FF(Q) {
  return Q;
}
function NF(Q) {
}
function OF(Q) {
  throw Error();
}
function DF(Q) {
}
function r9(Q) {
  let X = Object.values(Q).filter(($) => typeof $ === "number");
  return Object.entries(Q).filter(([$, J]) => X.indexOf(+$) === -1).map(([$, J]) => J);
}
function W8(Q, X = "|") {
  return Q.map((Y) => H8(Y)).join(X);
}
function JX(Q, X) {
  if (typeof X === "bigint") return X.toString();
  return X;
}
function t9(Q) {
  return { get value() {
    {
      let Y = Q();
      return Object.defineProperty(this, "value", { value: Y }), Y;
    }
    throw Error("cached value already set");
  } };
}
function a9(Q) {
  return Q === null || Q === void 0;
}
function s9(Q) {
  let X = Q.startsWith("^") ? 1 : 0, Y = Q.endsWith("$") ? Q.length - 1 : Q.length;
  return Q.slice(X, Y);
}
function WX(Q, X) {
  let Y = (Q.toString().split(".")[1] || "").length, $ = (X.toString().split(".")[1] || "").length, J = Y > $ ? Y : $, W = Number.parseInt(Q.toFixed(J).replace(".", "")), G = Number.parseInt(X.toFixed(J).replace(".", ""));
  return W % G / 10 ** J;
}
function $0(Q, X, Y) {
  Object.defineProperty(Q, X, { get() {
    {
      let J = Y();
      return Q[X] = J, J;
    }
    throw Error("cached value already set");
  }, set(J) {
    Object.defineProperty(Q, X, { value: J });
  }, configurable: true });
}
function GX(Q, X, Y) {
  Object.defineProperty(Q, X, { value: Y, writable: true, enumerable: true, configurable: true });
}
function MF(Q, X) {
  if (!X) return Q;
  return X.reduce((Y, $) => Y?.[$], Q);
}
function wF(Q) {
  let X = Object.keys(Q), Y = X.map(($) => Q[$]);
  return Promise.all(Y).then(($) => {
    let J = {};
    for (let W = 0; W < X.length; W++) J[X[W]] = $[W];
    return J;
  });
}
function AF(Q = 10) {
  let Y = "";
  for (let $ = 0; $ < Q; $++) Y += "abcdefghijklmnopqrstuvwxyz"[Math.floor(Math.random() * 26)];
  return Y;
}
function M6(Q) {
  return JSON.stringify(Q);
}
var G8 = Error.captureStackTrace ? Error.captureStackTrace : (...Q) => {
};
function K9(Q) {
  return typeof Q === "object" && Q !== null && !Array.isArray(Q);
}
var HX = t9(() => {
  if (typeof navigator < "u" && navigator?.userAgent?.includes("Cloudflare")) return false;
  try {
    return new Function(""), true;
  } catch (Q) {
    return false;
  }
});
function V9(Q) {
  if (K9(Q) === false) return false;
  let X = Q.constructor;
  if (X === void 0) return true;
  let Y = X.prototype;
  if (K9(Y) === false) return false;
  if (Object.prototype.hasOwnProperty.call(Y, "isPrototypeOf") === false) return false;
  return true;
}
function jF(Q) {
  let X = 0;
  for (let Y in Q) if (Object.prototype.hasOwnProperty.call(Q, Y)) X++;
  return X;
}
var RF = (Q) => {
  let X = typeof Q;
  switch (X) {
    case "undefined":
      return "undefined";
    case "string":
      return "string";
    case "number":
      return Number.isNaN(Q) ? "nan" : "number";
    case "boolean":
      return "boolean";
    case "function":
      return "function";
    case "bigint":
      return "bigint";
    case "symbol":
      return "symbol";
    case "object":
      if (Array.isArray(Q)) return "array";
      if (Q === null) return "null";
      if (Q.then && typeof Q.then === "function" && Q.catch && typeof Q.catch === "function") return "promise";
      if (typeof Map < "u" && Q instanceof Map) return "map";
      if (typeof Set < "u" && Q instanceof Set) return "set";
      if (typeof Date < "u" && Q instanceof Date) return "date";
      if (typeof File < "u" && Q instanceof File) return "file";
      return "object";
    default:
      throw Error(`Unknown data type: ${X}`);
  }
};
var BX = /* @__PURE__ */ new Set(["string", "number", "symbol"]);
var jJ = /* @__PURE__ */ new Set(["string", "number", "bigint", "boolean", "symbol", "undefined"]);
function o1(Q) {
  return Q.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
function c0(Q, X, Y) {
  let $ = new Q._zod.constr(X ?? Q._zod.def);
  if (!X || Y?.parent) $._zod.parent = Q;
  return $;
}
function y(Q) {
  let X = Q;
  if (!X) return {};
  if (typeof X === "string") return { error: () => X };
  if (X?.message !== void 0) {
    if (X?.error !== void 0) throw Error("Cannot specify both `message` and `error` params");
    X.error = X.message;
  }
  if (delete X.message, typeof X.error === "string") return { ...X, error: () => X.error };
  return X;
}
function IF(Q) {
  let X;
  return new Proxy({}, { get(Y, $, J) {
    return X ?? (X = Q()), Reflect.get(X, $, J);
  }, set(Y, $, J, W) {
    return X ?? (X = Q()), Reflect.set(X, $, J, W);
  }, has(Y, $) {
    return X ?? (X = Q()), Reflect.has(X, $);
  }, deleteProperty(Y, $) {
    return X ?? (X = Q()), Reflect.deleteProperty(X, $);
  }, ownKeys(Y) {
    return X ?? (X = Q()), Reflect.ownKeys(X);
  }, getOwnPropertyDescriptor(Y, $) {
    return X ?? (X = Q()), Reflect.getOwnPropertyDescriptor(X, $);
  }, defineProperty(Y, $, J) {
    return X ?? (X = Q()), Reflect.defineProperty(X, $, J);
  } });
}
function H8(Q) {
  if (typeof Q === "bigint") return Q.toString() + "n";
  if (typeof Q === "string") return `"${Q}"`;
  return `${Q}`;
}
function zX(Q) {
  return Object.keys(Q).filter((X) => {
    return Q[X]._zod.optin === "optional" && Q[X]._zod.optout === "optional";
  });
}
var KX = { safeint: [Number.MIN_SAFE_INTEGER, Number.MAX_SAFE_INTEGER], int32: [-2147483648, 2147483647], uint32: [0, 4294967295], float32: [-34028234663852886e22, 34028234663852886e22], float64: [-Number.MAX_VALUE, Number.MAX_VALUE] };
var RJ = { int64: [BigInt("-9223372036854775808"), BigInt("9223372036854775807")], uint64: [BigInt(0), BigInt("18446744073709551615")] };
function bF(Q, X) {
  let Y = {}, $ = Q._zod.def;
  for (let J in X) {
    if (!(J in $.shape)) throw Error(`Unrecognized key: "${J}"`);
    if (!X[J]) continue;
    Y[J] = $.shape[J];
  }
  return c0(Q, { ...Q._zod.def, shape: Y, checks: [] });
}
function EF(Q, X) {
  let Y = { ...Q._zod.def.shape }, $ = Q._zod.def;
  for (let J in X) {
    if (!(J in $.shape)) throw Error(`Unrecognized key: "${J}"`);
    if (!X[J]) continue;
    delete Y[J];
  }
  return c0(Q, { ...Q._zod.def, shape: Y, checks: [] });
}
function PF(Q, X) {
  if (!V9(X)) throw Error("Invalid input to extend: expected a plain object");
  let Y = { ...Q._zod.def, get shape() {
    let $ = { ...Q._zod.def.shape, ...X };
    return GX(this, "shape", $), $;
  }, checks: [] };
  return c0(Q, Y);
}
function ZF(Q, X) {
  return c0(Q, { ...Q._zod.def, get shape() {
    let Y = { ...Q._zod.def.shape, ...X._zod.def.shape };
    return GX(this, "shape", Y), Y;
  }, catchall: X._zod.def.catchall, checks: [] });
}
function CF(Q, X, Y) {
  let $ = X._zod.def.shape, J = { ...$ };
  if (Y) for (let W in Y) {
    if (!(W in $)) throw Error(`Unrecognized key: "${W}"`);
    if (!Y[W]) continue;
    J[W] = Q ? new Q({ type: "optional", innerType: $[W] }) : $[W];
  }
  else for (let W in $) J[W] = Q ? new Q({ type: "optional", innerType: $[W] }) : $[W];
  return c0(X, { ...X._zod.def, shape: J, checks: [] });
}
function SF(Q, X, Y) {
  let $ = X._zod.def.shape, J = { ...$ };
  if (Y) for (let W in Y) {
    if (!(W in J)) throw Error(`Unrecognized key: "${W}"`);
    if (!Y[W]) continue;
    J[W] = new Q({ type: "nonoptional", innerType: $[W] });
  }
  else for (let W in $) J[W] = new Q({ type: "nonoptional", innerType: $[W] });
  return c0(X, { ...X._zod.def, shape: J, checks: [] });
}
function w6(Q, X = 0) {
  for (let Y = X; Y < Q.issues.length; Y++) if (Q.issues[Y]?.continue !== true) return true;
  return false;
}
function j1(Q, X) {
  return X.map((Y) => {
    var $;
    return ($ = Y).path ?? ($.path = []), Y.path.unshift(Q), Y;
  });
}
function o9(Q) {
  return typeof Q === "string" ? Q : Q?.message;
}
function G1(Q, X, Y) {
  let $ = { ...Q, path: Q.path ?? [] };
  if (!Q.message) {
    let J = o9(Q.inst?._zod.def?.error?.(Q)) ?? o9(X?.error?.(Q)) ?? o9(Y.customError?.(Q)) ?? o9(Y.localeError?.(Q)) ?? "Invalid input";
    $.message = J;
  }
  if (delete $.inst, delete $.continue, !X?.reportInput) delete $.input;
  return $;
}
function IJ(Q) {
  if (Q instanceof Set) return "set";
  if (Q instanceof Map) return "map";
  if (Q instanceof File) return "file";
  return "unknown";
}
function e9(Q) {
  if (Array.isArray(Q)) return "array";
  if (typeof Q === "string") return "string";
  return "unknown";
}
function VX(...Q) {
  let [X, Y, $] = Q;
  if (typeof X === "string") return { message: X, code: "custom", input: Y, inst: $ };
  return { ...X };
}
function _F(Q) {
  return Object.entries(Q).filter(([X, Y]) => {
    return Number.isNaN(Number.parseInt(X, 10));
  }).map((X) => X[1]);
}
var bJ = class {
  constructor(...Q) {
  }
};
var EJ = (Q, X) => {
  Q.name = "$ZodError", Object.defineProperty(Q, "_zod", { value: Q._zod, enumerable: false }), Object.defineProperty(Q, "issues", { value: X, enumerable: false }), Object.defineProperty(Q, "message", { get() {
    return JSON.stringify(X, JX, 2);
  }, enumerable: true });
};
var B8 = D("$ZodError", EJ);
var Q4 = D("$ZodError", EJ, { Parent: Error });
function qX(Q, X = (Y) => Y.message) {
  let Y = {}, $ = [];
  for (let J of Q.issues) if (J.path.length > 0) Y[J.path[0]] = Y[J.path[0]] || [], Y[J.path[0]].push(X(J));
  else $.push(X(J));
  return { formErrors: $, fieldErrors: Y };
}
function UX(Q, X) {
  let Y = X || function(W) {
    return W.message;
  }, $ = { _errors: [] }, J = (W) => {
    for (let G of W.issues) if (G.code === "invalid_union" && G.errors.length) G.errors.map((H) => J({ issues: H }));
    else if (G.code === "invalid_key") J({ issues: G.issues });
    else if (G.code === "invalid_element") J({ issues: G.issues });
    else if (G.path.length === 0) $._errors.push(Y(G));
    else {
      let H = $, B = 0;
      while (B < G.path.length) {
        let z = G.path[B];
        if (B !== G.path.length - 1) H[z] = H[z] || { _errors: [] };
        else H[z] = H[z] || { _errors: [] }, H[z]._errors.push(Y(G));
        H = H[z], B++;
      }
    }
  };
  return J(Q), $;
}
var LX = (Q) => (X, Y, $, J) => {
  let W = $ ? Object.assign($, { async: false }) : { async: false }, G = X._zod.run({ value: Y, issues: [] }, W);
  if (G instanceof Promise) throw new n1();
  if (G.issues.length) {
    let H = new (J?.Err ?? Q)(G.issues.map((B) => G1(B, W, l0())));
    throw G8(H, J?.callee), H;
  }
  return G.value;
};
var FX = LX(Q4);
var NX = (Q) => async (X, Y, $, J) => {
  let W = $ ? Object.assign($, { async: true }) : { async: true }, G = X._zod.run({ value: Y, issues: [] }, W);
  if (G instanceof Promise) G = await G;
  if (G.issues.length) {
    let H = new (J?.Err ?? Q)(G.issues.map((B) => G1(B, W, l0())));
    throw G8(H, J?.callee), H;
  }
  return G.value;
};
var OX = NX(Q4);
var DX = (Q) => (X, Y, $) => {
  let J = $ ? { ...$, async: false } : { async: false }, W = X._zod.run({ value: Y, issues: [] }, J);
  if (W instanceof Promise) throw new n1();
  return W.issues.length ? { success: false, error: new (Q ?? B8)(W.issues.map((G) => G1(G, J, l0()))) } : { success: true, data: W.value };
};
var A6 = DX(Q4);
var MX = (Q) => async (X, Y, $) => {
  let J = $ ? Object.assign($, { async: true }) : { async: true }, W = X._zod.run({ value: Y, issues: [] }, J);
  if (W instanceof Promise) W = await W;
  return W.issues.length ? { success: false, error: new Q(W.issues.map((G) => G1(G, J, l0()))) } : { success: true, data: W.value };
};
var j6 = MX(Q4);
var PJ = /^[cC][^\s-]{8,}$/;
var ZJ = /^[0-9a-z]+$/;
var CJ = /^[0-9A-HJKMNP-TV-Za-hjkmnp-tv-z]{26}$/;
var SJ = /^[0-9a-vA-V]{20}$/;
var _J = /^[A-Za-z0-9]{27}$/;
var kJ = /^[a-zA-Z0-9_-]{21}$/;
var vJ = /^P(?:(\d+W)|(?!.*W)(?=\d|T\d)(\d+Y)?(\d+M)?(\d+D)?(T(?=\d)(\d+H)?(\d+M)?(\d+([.,]\d+)?S)?)?)$/;
var TJ = /^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$/;
var wX = (Q) => {
  if (!Q) return /^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}|00000000-0000-0000-0000-000000000000)$/;
  return new RegExp(`^([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-${Q}[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12})$`);
};
var xJ = /^(?!\.)(?!.*\.\.)([A-Za-z0-9_'+\-\.]*)[A-Za-z0-9_+-]@([A-Za-z0-9][A-Za-z0-9\-]*\.)+[A-Za-z]{2,}$/;
function yJ() {
  return new RegExp("^(\\p{Extended_Pictographic}|\\p{Emoji_Component})+$", "u");
}
var gJ = /^(?:(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(?:25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])$/;
var hJ = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|::|([0-9a-fA-F]{1,4})?::([0-9a-fA-F]{1,4}:?){0,6})$/;
var fJ = /^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9][0-9]|[0-9])\/([0-9]|[1-2][0-9]|3[0-2])$/;
var uJ = /^(([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|::|([0-9a-fA-F]{1,4})?::([0-9a-fA-F]{1,4}:?){0,6})\/(12[0-8]|1[01][0-9]|[1-9]?[0-9])$/;
var mJ = /^$|^(?:[0-9a-zA-Z+/]{4})*(?:(?:[0-9a-zA-Z+/]{2}==)|(?:[0-9a-zA-Z+/]{3}=))?$/;
var AX = /^[A-Za-z0-9_-]*$/;
var lJ = /^([a-zA-Z0-9-]+\.)*[a-zA-Z0-9-]+$/;
var cJ = /^\+(?:[0-9]){6,14}[0-9]$/;
var pJ = "(?:(?:\\d\\d[2468][048]|\\d\\d[13579][26]|\\d\\d0[48]|[02468][048]00|[13579][26]00)-02-29|\\d{4}-(?:(?:0[13578]|1[02])-(?:0[1-9]|[12]\\d|3[01])|(?:0[469]|11)-(?:0[1-9]|[12]\\d|30)|(?:02)-(?:0[1-9]|1\\d|2[0-8])))";
var dJ = new RegExp(`^${pJ}$`);
function iJ(Q) {
  return typeof Q.precision === "number" ? Q.precision === -1 ? "(?:[01]\\d|2[0-3]):[0-5]\\d" : Q.precision === 0 ? "(?:[01]\\d|2[0-3]):[0-5]\\d:[0-5]\\d" : `(?:[01]\\d|2[0-3]):[0-5]\\d:[0-5]\\d\\.\\d{${Q.precision}}` : "(?:[01]\\d|2[0-3]):[0-5]\\d(?::[0-5]\\d(?:\\.\\d+)?)?";
}
function nJ(Q) {
  return new RegExp(`^${iJ(Q)}$`);
}
function oJ(Q) {
  let X = iJ({ precision: Q.precision }), Y = ["Z"];
  if (Q.local) Y.push("");
  if (Q.offset) Y.push("([+-]\\d{2}:\\d{2})");
  let $ = `${X}(?:${Y.join("|")})`;
  return new RegExp(`^${pJ}T(?:${$})$`);
}
var rJ = (Q) => {
  let X = Q ? `[\\s\\S]{${Q?.minimum ?? 0},${Q?.maximum ?? ""}}` : "[\\s\\S]*";
  return new RegExp(`^${X}$`);
};
var tJ = /^\d+$/;
var aJ = /^-?\d+(?:\.\d+)?/i;
var sJ = /true|false/i;
var eJ = /null/i;
var QW = /^[^A-Z]*$/;
var XW = /^[^a-z]*$/;
var j0 = D("$ZodCheck", (Q, X) => {
  var Y;
  Q._zod ?? (Q._zod = {}), Q._zod.def = X, (Y = Q._zod).onattach ?? (Y.onattach = []);
});
var YW = { number: "number", bigint: "bigint", object: "date" };
var jX = D("$ZodCheckLessThan", (Q, X) => {
  j0.init(Q, X);
  let Y = YW[typeof X.value];
  Q._zod.onattach.push(($) => {
    let J = $._zod.bag, W = (X.inclusive ? J.maximum : J.exclusiveMaximum) ?? Number.POSITIVE_INFINITY;
    if (X.value < W) if (X.inclusive) J.maximum = X.value;
    else J.exclusiveMaximum = X.value;
  }), Q._zod.check = ($) => {
    if (X.inclusive ? $.value <= X.value : $.value < X.value) return;
    $.issues.push({ origin: Y, code: "too_big", maximum: X.value, input: $.value, inclusive: X.inclusive, inst: Q, continue: !X.abort });
  };
});
var RX = D("$ZodCheckGreaterThan", (Q, X) => {
  j0.init(Q, X);
  let Y = YW[typeof X.value];
  Q._zod.onattach.push(($) => {
    let J = $._zod.bag, W = (X.inclusive ? J.minimum : J.exclusiveMinimum) ?? Number.NEGATIVE_INFINITY;
    if (X.value > W) if (X.inclusive) J.minimum = X.value;
    else J.exclusiveMinimum = X.value;
  }), Q._zod.check = ($) => {
    if (X.inclusive ? $.value >= X.value : $.value > X.value) return;
    $.issues.push({ origin: Y, code: "too_small", minimum: X.value, input: $.value, inclusive: X.inclusive, inst: Q, continue: !X.abort });
  };
});
var $W = D("$ZodCheckMultipleOf", (Q, X) => {
  j0.init(Q, X), Q._zod.onattach.push((Y) => {
    var $;
    ($ = Y._zod.bag).multipleOf ?? ($.multipleOf = X.value);
  }), Q._zod.check = (Y) => {
    if (typeof Y.value !== typeof X.value) throw Error("Cannot mix number and bigint in multiple_of check.");
    if (typeof Y.value === "bigint" ? Y.value % X.value === BigInt(0) : WX(Y.value, X.value) === 0) return;
    Y.issues.push({ origin: typeof Y.value, code: "not_multiple_of", divisor: X.value, input: Y.value, inst: Q, continue: !X.abort });
  };
});
var JW = D("$ZodCheckNumberFormat", (Q, X) => {
  j0.init(Q, X), X.format = X.format || "float64";
  let Y = X.format?.includes("int"), $ = Y ? "int" : "number", [J, W] = KX[X.format];
  Q._zod.onattach.push((G) => {
    let H = G._zod.bag;
    if (H.format = X.format, H.minimum = J, H.maximum = W, Y) H.pattern = tJ;
  }), Q._zod.check = (G) => {
    let H = G.value;
    if (Y) {
      if (!Number.isInteger(H)) {
        G.issues.push({ expected: $, format: X.format, code: "invalid_type", input: H, inst: Q });
        return;
      }
      if (!Number.isSafeInteger(H)) {
        if (H > 0) G.issues.push({ input: H, code: "too_big", maximum: Number.MAX_SAFE_INTEGER, note: "Integers must be within the safe integer range.", inst: Q, origin: $, continue: !X.abort });
        else G.issues.push({ input: H, code: "too_small", minimum: Number.MIN_SAFE_INTEGER, note: "Integers must be within the safe integer range.", inst: Q, origin: $, continue: !X.abort });
        return;
      }
    }
    if (H < J) G.issues.push({ origin: "number", input: H, code: "too_small", minimum: J, inclusive: true, inst: Q, continue: !X.abort });
    if (H > W) G.issues.push({ origin: "number", input: H, code: "too_big", maximum: W, inst: Q });
  };
});
var WW = D("$ZodCheckMaxLength", (Q, X) => {
  j0.init(Q, X), Q._zod.when = (Y) => {
    let $ = Y.value;
    return !a9($) && $.length !== void 0;
  }, Q._zod.onattach.push((Y) => {
    let $ = Y._zod.bag.maximum ?? Number.POSITIVE_INFINITY;
    if (X.maximum < $) Y._zod.bag.maximum = X.maximum;
  }), Q._zod.check = (Y) => {
    let $ = Y.value;
    if ($.length <= X.maximum) return;
    let W = e9($);
    Y.issues.push({ origin: W, code: "too_big", maximum: X.maximum, inclusive: true, input: $, inst: Q, continue: !X.abort });
  };
});
var GW = D("$ZodCheckMinLength", (Q, X) => {
  j0.init(Q, X), Q._zod.when = (Y) => {
    let $ = Y.value;
    return !a9($) && $.length !== void 0;
  }, Q._zod.onattach.push((Y) => {
    let $ = Y._zod.bag.minimum ?? Number.NEGATIVE_INFINITY;
    if (X.minimum > $) Y._zod.bag.minimum = X.minimum;
  }), Q._zod.check = (Y) => {
    let $ = Y.value;
    if ($.length >= X.minimum) return;
    let W = e9($);
    Y.issues.push({ origin: W, code: "too_small", minimum: X.minimum, inclusive: true, input: $, inst: Q, continue: !X.abort });
  };
});
var HW = D("$ZodCheckLengthEquals", (Q, X) => {
  j0.init(Q, X), Q._zod.when = (Y) => {
    let $ = Y.value;
    return !a9($) && $.length !== void 0;
  }, Q._zod.onattach.push((Y) => {
    let $ = Y._zod.bag;
    $.minimum = X.length, $.maximum = X.length, $.length = X.length;
  }), Q._zod.check = (Y) => {
    let $ = Y.value, J = $.length;
    if (J === X.length) return;
    let W = e9($), G = J > X.length;
    Y.issues.push({ origin: W, ...G ? { code: "too_big", maximum: X.length } : { code: "too_small", minimum: X.length }, inclusive: true, exact: true, input: Y.value, inst: Q, continue: !X.abort });
  };
});
var X4 = D("$ZodCheckStringFormat", (Q, X) => {
  var Y, $;
  if (j0.init(Q, X), Q._zod.onattach.push((J) => {
    let W = J._zod.bag;
    if (W.format = X.format, X.pattern) W.patterns ?? (W.patterns = /* @__PURE__ */ new Set()), W.patterns.add(X.pattern);
  }), X.pattern) (Y = Q._zod).check ?? (Y.check = (J) => {
    if (X.pattern.lastIndex = 0, X.pattern.test(J.value)) return;
    J.issues.push({ origin: "string", code: "invalid_format", format: X.format, input: J.value, ...X.pattern ? { pattern: X.pattern.toString() } : {}, inst: Q, continue: !X.abort });
  });
  else ($ = Q._zod).check ?? ($.check = () => {
  });
});
var BW = D("$ZodCheckRegex", (Q, X) => {
  X4.init(Q, X), Q._zod.check = (Y) => {
    if (X.pattern.lastIndex = 0, X.pattern.test(Y.value)) return;
    Y.issues.push({ origin: "string", code: "invalid_format", format: "regex", input: Y.value, pattern: X.pattern.toString(), inst: Q, continue: !X.abort });
  };
});
var zW = D("$ZodCheckLowerCase", (Q, X) => {
  X.pattern ?? (X.pattern = QW), X4.init(Q, X);
});
var KW = D("$ZodCheckUpperCase", (Q, X) => {
  X.pattern ?? (X.pattern = XW), X4.init(Q, X);
});
var VW = D("$ZodCheckIncludes", (Q, X) => {
  j0.init(Q, X);
  let Y = o1(X.includes), $ = new RegExp(typeof X.position === "number" ? `^.{${X.position}}${Y}` : Y);
  X.pattern = $, Q._zod.onattach.push((J) => {
    let W = J._zod.bag;
    W.patterns ?? (W.patterns = /* @__PURE__ */ new Set()), W.patterns.add($);
  }), Q._zod.check = (J) => {
    if (J.value.includes(X.includes, X.position)) return;
    J.issues.push({ origin: "string", code: "invalid_format", format: "includes", includes: X.includes, input: J.value, inst: Q, continue: !X.abort });
  };
});
var qW = D("$ZodCheckStartsWith", (Q, X) => {
  j0.init(Q, X);
  let Y = new RegExp(`^${o1(X.prefix)}.*`);
  X.pattern ?? (X.pattern = Y), Q._zod.onattach.push(($) => {
    let J = $._zod.bag;
    J.patterns ?? (J.patterns = /* @__PURE__ */ new Set()), J.patterns.add(Y);
  }), Q._zod.check = ($) => {
    if ($.value.startsWith(X.prefix)) return;
    $.issues.push({ origin: "string", code: "invalid_format", format: "starts_with", prefix: X.prefix, input: $.value, inst: Q, continue: !X.abort });
  };
});
var UW = D("$ZodCheckEndsWith", (Q, X) => {
  j0.init(Q, X);
  let Y = new RegExp(`.*${o1(X.suffix)}$`);
  X.pattern ?? (X.pattern = Y), Q._zod.onattach.push(($) => {
    let J = $._zod.bag;
    J.patterns ?? (J.patterns = /* @__PURE__ */ new Set()), J.patterns.add(Y);
  }), Q._zod.check = ($) => {
    if ($.value.endsWith(X.suffix)) return;
    $.issues.push({ origin: "string", code: "invalid_format", format: "ends_with", suffix: X.suffix, input: $.value, inst: Q, continue: !X.abort });
  };
});
var LW = D("$ZodCheckOverwrite", (Q, X) => {
  j0.init(Q, X), Q._zod.check = (Y) => {
    Y.value = X.tx(Y.value);
  };
});
var IX = class {
  constructor(Q = []) {
    if (this.content = [], this.indent = 0, this) this.args = Q;
  }
  indented(Q) {
    this.indent += 1, Q(this), this.indent -= 1;
  }
  write(Q) {
    if (typeof Q === "function") {
      Q(this, { execution: "sync" }), Q(this, { execution: "async" });
      return;
    }
    let Y = Q.split(`
`).filter((W) => W), $ = Math.min(...Y.map((W) => W.length - W.trimStart().length)), J = Y.map((W) => W.slice($)).map((W) => " ".repeat(this.indent * 2) + W);
    for (let W of J) this.content.push(W);
  }
  compile() {
    let Q = Function, X = this?.args, $ = [...(this?.content ?? [""]).map((J) => `  ${J}`)];
    return new Q(...X, $.join(`
`));
  }
};
var NW = { major: 4, minor: 0, patch: 0 };
var e = D("$ZodType", (Q, X) => {
  var Y;
  Q ?? (Q = {}), Q._zod.def = X, Q._zod.bag = Q._zod.bag || {}, Q._zod.version = NW;
  let $ = [...Q._zod.def.checks ?? []];
  if (Q._zod.traits.has("$ZodCheck")) $.unshift(Q);
  for (let J of $) for (let W of J._zod.onattach) W(Q);
  if ($.length === 0) (Y = Q._zod).deferred ?? (Y.deferred = []), Q._zod.deferred?.push(() => {
    Q._zod.run = Q._zod.parse;
  });
  else {
    let J = (W, G, H) => {
      let B = w6(W), z;
      for (let K of G) {
        if (K._zod.when) {
          if (!K._zod.when(W)) continue;
        } else if (B) continue;
        let U = W.issues.length, q = K._zod.check(W);
        if (q instanceof Promise && H?.async === false) throw new n1();
        if (z || q instanceof Promise) z = (z ?? Promise.resolve()).then(async () => {
          if (await q, W.issues.length === U) return;
          if (!B) B = w6(W, U);
        });
        else {
          if (W.issues.length === U) continue;
          if (!B) B = w6(W, U);
        }
      }
      if (z) return z.then(() => {
        return W;
      });
      return W;
    };
    Q._zod.run = (W, G) => {
      let H = Q._zod.parse(W, G);
      if (H instanceof Promise) {
        if (G.async === false) throw new n1();
        return H.then((B) => J(B, $, G));
      }
      return J(H, $, G);
    };
  }
  Q["~standard"] = { validate: (J) => {
    try {
      let W = A6(Q, J);
      return W.success ? { value: W.data } : { issues: W.error?.issues };
    } catch (W) {
      return j6(Q, J).then((G) => G.success ? { value: G.data } : { issues: G.error?.issues });
    }
  }, vendor: "zod", version: 1 };
});
var Y4 = D("$ZodString", (Q, X) => {
  e.init(Q, X), Q._zod.pattern = [...Q?._zod.bag?.patterns ?? []].pop() ?? rJ(Q._zod.bag), Q._zod.parse = (Y, $) => {
    if (X.coerce) try {
      Y.value = String(Y.value);
    } catch (J) {
    }
    if (typeof Y.value === "string") return Y;
    return Y.issues.push({ expected: "string", code: "invalid_type", input: Y.value, inst: Q }), Y;
  };
});
var J0 = D("$ZodStringFormat", (Q, X) => {
  X4.init(Q, X), Y4.init(Q, X);
});
var EX = D("$ZodGUID", (Q, X) => {
  X.pattern ?? (X.pattern = TJ), J0.init(Q, X);
});
var PX = D("$ZodUUID", (Q, X) => {
  if (X.version) {
    let $ = { v1: 1, v2: 2, v3: 3, v4: 4, v5: 5, v6: 6, v7: 7, v8: 8 }[X.version];
    if ($ === void 0) throw Error(`Invalid UUID version: "${X.version}"`);
    X.pattern ?? (X.pattern = wX($));
  } else X.pattern ?? (X.pattern = wX());
  J0.init(Q, X);
});
var ZX = D("$ZodEmail", (Q, X) => {
  X.pattern ?? (X.pattern = xJ), J0.init(Q, X);
});
var CX = D("$ZodURL", (Q, X) => {
  J0.init(Q, X), Q._zod.check = (Y) => {
    try {
      let $ = Y.value, J = new URL($), W = J.href;
      if (X.hostname) {
        if (X.hostname.lastIndex = 0, !X.hostname.test(J.hostname)) Y.issues.push({ code: "invalid_format", format: "url", note: "Invalid hostname", pattern: lJ.source, input: Y.value, inst: Q, continue: !X.abort });
      }
      if (X.protocol) {
        if (X.protocol.lastIndex = 0, !X.protocol.test(J.protocol.endsWith(":") ? J.protocol.slice(0, -1) : J.protocol)) Y.issues.push({ code: "invalid_format", format: "url", note: "Invalid protocol", pattern: X.protocol.source, input: Y.value, inst: Q, continue: !X.abort });
      }
      if (!$.endsWith("/") && W.endsWith("/")) Y.value = W.slice(0, -1);
      else Y.value = W;
      return;
    } catch ($) {
      Y.issues.push({ code: "invalid_format", format: "url", input: Y.value, inst: Q, continue: !X.abort });
    }
  };
});
var SX = D("$ZodEmoji", (Q, X) => {
  X.pattern ?? (X.pattern = yJ()), J0.init(Q, X);
});
var _X = D("$ZodNanoID", (Q, X) => {
  X.pattern ?? (X.pattern = kJ), J0.init(Q, X);
});
var kX = D("$ZodCUID", (Q, X) => {
  X.pattern ?? (X.pattern = PJ), J0.init(Q, X);
});
var vX = D("$ZodCUID2", (Q, X) => {
  X.pattern ?? (X.pattern = ZJ), J0.init(Q, X);
});
var TX = D("$ZodULID", (Q, X) => {
  X.pattern ?? (X.pattern = CJ), J0.init(Q, X);
});
var xX = D("$ZodXID", (Q, X) => {
  X.pattern ?? (X.pattern = SJ), J0.init(Q, X);
});
var yX = D("$ZodKSUID", (Q, X) => {
  X.pattern ?? (X.pattern = _J), J0.init(Q, X);
});
var EW = D("$ZodISODateTime", (Q, X) => {
  X.pattern ?? (X.pattern = oJ(X)), J0.init(Q, X);
});
var PW = D("$ZodISODate", (Q, X) => {
  X.pattern ?? (X.pattern = dJ), J0.init(Q, X);
});
var ZW = D("$ZodISOTime", (Q, X) => {
  X.pattern ?? (X.pattern = nJ(X)), J0.init(Q, X);
});
var CW = D("$ZodISODuration", (Q, X) => {
  X.pattern ?? (X.pattern = vJ), J0.init(Q, X);
});
var gX = D("$ZodIPv4", (Q, X) => {
  X.pattern ?? (X.pattern = gJ), J0.init(Q, X), Q._zod.onattach.push((Y) => {
    let $ = Y._zod.bag;
    $.format = "ipv4";
  });
});
var hX = D("$ZodIPv6", (Q, X) => {
  X.pattern ?? (X.pattern = hJ), J0.init(Q, X), Q._zod.onattach.push((Y) => {
    let $ = Y._zod.bag;
    $.format = "ipv6";
  }), Q._zod.check = (Y) => {
    try {
      new URL(`http://[${Y.value}]`);
    } catch {
      Y.issues.push({ code: "invalid_format", format: "ipv6", input: Y.value, inst: Q, continue: !X.abort });
    }
  };
});
var fX = D("$ZodCIDRv4", (Q, X) => {
  X.pattern ?? (X.pattern = fJ), J0.init(Q, X);
});
var uX = D("$ZodCIDRv6", (Q, X) => {
  X.pattern ?? (X.pattern = uJ), J0.init(Q, X), Q._zod.check = (Y) => {
    let [$, J] = Y.value.split("/");
    try {
      if (!J) throw Error();
      let W = Number(J);
      if (`${W}` !== J) throw Error();
      if (W < 0 || W > 128) throw Error();
      new URL(`http://[${$}]`);
    } catch {
      Y.issues.push({ code: "invalid_format", format: "cidrv6", input: Y.value, inst: Q, continue: !X.abort });
    }
  };
});
function SW(Q) {
  if (Q === "") return true;
  if (Q.length % 4 !== 0) return false;
  try {
    return atob(Q), true;
  } catch {
    return false;
  }
}
var mX = D("$ZodBase64", (Q, X) => {
  X.pattern ?? (X.pattern = mJ), J0.init(Q, X), Q._zod.onattach.push((Y) => {
    Y._zod.bag.contentEncoding = "base64";
  }), Q._zod.check = (Y) => {
    if (SW(Y.value)) return;
    Y.issues.push({ code: "invalid_format", format: "base64", input: Y.value, inst: Q, continue: !X.abort });
  };
});
function vF(Q) {
  if (!AX.test(Q)) return false;
  let X = Q.replace(/[-_]/g, ($) => $ === "-" ? "+" : "/"), Y = X.padEnd(Math.ceil(X.length / 4) * 4, "=");
  return SW(Y);
}
var lX = D("$ZodBase64URL", (Q, X) => {
  X.pattern ?? (X.pattern = AX), J0.init(Q, X), Q._zod.onattach.push((Y) => {
    Y._zod.bag.contentEncoding = "base64url";
  }), Q._zod.check = (Y) => {
    if (vF(Y.value)) return;
    Y.issues.push({ code: "invalid_format", format: "base64url", input: Y.value, inst: Q, continue: !X.abort });
  };
});
var cX = D("$ZodE164", (Q, X) => {
  X.pattern ?? (X.pattern = cJ), J0.init(Q, X);
});
function TF(Q, X = null) {
  try {
    let Y = Q.split(".");
    if (Y.length !== 3) return false;
    let [$] = Y;
    if (!$) return false;
    let J = JSON.parse(atob($));
    if ("typ" in J && J?.typ !== "JWT") return false;
    if (!J.alg) return false;
    if (X && (!("alg" in J) || J.alg !== X)) return false;
    return true;
  } catch {
    return false;
  }
}
var pX = D("$ZodJWT", (Q, X) => {
  J0.init(Q, X), Q._zod.check = (Y) => {
    if (TF(Y.value, X.alg)) return;
    Y.issues.push({ code: "invalid_format", format: "jwt", input: Y.value, inst: Q, continue: !X.abort });
  };
});
var V8 = D("$ZodNumber", (Q, X) => {
  e.init(Q, X), Q._zod.pattern = Q._zod.bag.pattern ?? aJ, Q._zod.parse = (Y, $) => {
    if (X.coerce) try {
      Y.value = Number(Y.value);
    } catch (G) {
    }
    let J = Y.value;
    if (typeof J === "number" && !Number.isNaN(J) && Number.isFinite(J)) return Y;
    let W = typeof J === "number" ? Number.isNaN(J) ? "NaN" : !Number.isFinite(J) ? "Infinity" : void 0 : void 0;
    return Y.issues.push({ expected: "number", code: "invalid_type", input: J, inst: Q, ...W ? { received: W } : {} }), Y;
  };
});
var dX = D("$ZodNumber", (Q, X) => {
  JW.init(Q, X), V8.init(Q, X);
});
var iX = D("$ZodBoolean", (Q, X) => {
  e.init(Q, X), Q._zod.pattern = sJ, Q._zod.parse = (Y, $) => {
    if (X.coerce) try {
      Y.value = Boolean(Y.value);
    } catch (W) {
    }
    let J = Y.value;
    if (typeof J === "boolean") return Y;
    return Y.issues.push({ expected: "boolean", code: "invalid_type", input: J, inst: Q }), Y;
  };
});
var nX = D("$ZodNull", (Q, X) => {
  e.init(Q, X), Q._zod.pattern = eJ, Q._zod.values = /* @__PURE__ */ new Set([null]), Q._zod.parse = (Y, $) => {
    let J = Y.value;
    if (J === null) return Y;
    return Y.issues.push({ expected: "null", code: "invalid_type", input: J, inst: Q }), Y;
  };
});
var oX = D("$ZodUnknown", (Q, X) => {
  e.init(Q, X), Q._zod.parse = (Y) => Y;
});
var rX = D("$ZodNever", (Q, X) => {
  e.init(Q, X), Q._zod.parse = (Y, $) => {
    return Y.issues.push({ expected: "never", code: "invalid_type", input: Y.value, inst: Q }), Y;
  };
});
function OW(Q, X, Y) {
  if (Q.issues.length) X.issues.push(...j1(Y, Q.issues));
  X.value[Y] = Q.value;
}
var tX = D("$ZodArray", (Q, X) => {
  e.init(Q, X), Q._zod.parse = (Y, $) => {
    let J = Y.value;
    if (!Array.isArray(J)) return Y.issues.push({ expected: "array", code: "invalid_type", input: J, inst: Q }), Y;
    Y.value = Array(J.length);
    let W = [];
    for (let G = 0; G < J.length; G++) {
      let H = J[G], B = X.element._zod.run({ value: H, issues: [] }, $);
      if (B instanceof Promise) W.push(B.then((z) => OW(z, Y, G)));
      else OW(B, Y, G);
    }
    if (W.length) return Promise.all(W).then(() => Y);
    return Y;
  };
});
function K8(Q, X, Y) {
  if (Q.issues.length) X.issues.push(...j1(Y, Q.issues));
  X.value[Y] = Q.value;
}
function DW(Q, X, Y, $) {
  if (Q.issues.length) if ($[Y] === void 0) if (Y in $) X.value[Y] = void 0;
  else X.value[Y] = Q.value;
  else X.issues.push(...j1(Y, Q.issues));
  else if (Q.value === void 0) {
    if (Y in $) X.value[Y] = void 0;
  } else X.value[Y] = Q.value;
}
var q8 = D("$ZodObject", (Q, X) => {
  e.init(Q, X);
  let Y = t9(() => {
    let U = Object.keys(X.shape);
    for (let V of U) if (!(X.shape[V] instanceof e)) throw Error(`Invalid element at key "${V}": expected a Zod schema`);
    let q = zX(X.shape);
    return { shape: X.shape, keys: U, keySet: new Set(U), numKeys: U.length, optionalKeys: new Set(q) };
  });
  $0(Q._zod, "propValues", () => {
    let U = X.shape, q = {};
    for (let V in U) {
      let L = U[V]._zod;
      if (L.values) {
        q[V] ?? (q[V] = /* @__PURE__ */ new Set());
        for (let F of L.values) q[V].add(F);
      }
    }
    return q;
  });
  let $ = (U) => {
    let q = new IX(["shape", "payload", "ctx"]), V = Y.value, L = (A) => {
      let R = M6(A);
      return `shape[${R}]._zod.run({ value: input[${R}], issues: [] }, ctx)`;
    };
    q.write("const input = payload.value;");
    let F = /* @__PURE__ */ Object.create(null), M = 0;
    for (let A of V.keys) F[A] = `key_${M++}`;
    q.write("const newResult = {}");
    for (let A of V.keys) if (V.optionalKeys.has(A)) {
      let R = F[A];
      q.write(`const ${R} = ${L(A)};`);
      let Z = M6(A);
      q.write(`
        if (${R}.issues.length) {
          if (input[${Z}] === undefined) {
            if (${Z} in input) {
              newResult[${Z}] = undefined;
            }
          } else {
            payload.issues = payload.issues.concat(
              ${R}.issues.map((iss) => ({
                ...iss,
                path: iss.path ? [${Z}, ...iss.path] : [${Z}],
              }))
            );
          }
        } else if (${R}.value === undefined) {
          if (${Z} in input) newResult[${Z}] = undefined;
        } else {
          newResult[${Z}] = ${R}.value;
        }
        `);
    } else {
      let R = F[A];
      q.write(`const ${R} = ${L(A)};`), q.write(`
          if (${R}.issues.length) payload.issues = payload.issues.concat(${R}.issues.map(iss => ({
            ...iss,
            path: iss.path ? [${M6(A)}, ...iss.path] : [${M6(A)}]
          })));`), q.write(`newResult[${M6(A)}] = ${R}.value`);
    }
    q.write("payload.value = newResult;"), q.write("return payload;");
    let O = q.compile();
    return (A, R) => O(U, A, R);
  }, J, W = K9, G = !J8.jitless, B = G && HX.value, z = X.catchall, K;
  Q._zod.parse = (U, q) => {
    K ?? (K = Y.value);
    let V = U.value;
    if (!W(V)) return U.issues.push({ expected: "object", code: "invalid_type", input: V, inst: Q }), U;
    let L = [];
    if (G && B && q?.async === false && q.jitless !== true) {
      if (!J) J = $(X.shape);
      U = J(U, q);
    } else {
      U.value = {};
      let R = K.shape;
      for (let Z of K.keys) {
        let C = R[Z], B0 = C._zod.run({ value: V[Z], issues: [] }, q), O0 = C._zod.optin === "optional" && C._zod.optout === "optional";
        if (B0 instanceof Promise) L.push(B0.then((d0) => O0 ? DW(d0, U, Z, V) : K8(d0, U, Z)));
        else if (O0) DW(B0, U, Z, V);
        else K8(B0, U, Z);
      }
    }
    if (!z) return L.length ? Promise.all(L).then(() => U) : U;
    let F = [], M = K.keySet, O = z._zod, A = O.def.type;
    for (let R of Object.keys(V)) {
      if (M.has(R)) continue;
      if (A === "never") {
        F.push(R);
        continue;
      }
      let Z = O.run({ value: V[R], issues: [] }, q);
      if (Z instanceof Promise) L.push(Z.then((C) => K8(C, U, R)));
      else K8(Z, U, R);
    }
    if (F.length) U.issues.push({ code: "unrecognized_keys", keys: F, input: V, inst: Q });
    if (!L.length) return U;
    return Promise.all(L).then(() => {
      return U;
    });
  };
});
function MW(Q, X, Y, $) {
  for (let J of Q) if (J.issues.length === 0) return X.value = J.value, X;
  return X.issues.push({ code: "invalid_union", input: X.value, inst: Y, errors: Q.map((J) => J.issues.map((W) => G1(W, $, l0()))) }), X;
}
var U8 = D("$ZodUnion", (Q, X) => {
  e.init(Q, X), $0(Q._zod, "optin", () => X.options.some((Y) => Y._zod.optin === "optional") ? "optional" : void 0), $0(Q._zod, "optout", () => X.options.some((Y) => Y._zod.optout === "optional") ? "optional" : void 0), $0(Q._zod, "values", () => {
    if (X.options.every((Y) => Y._zod.values)) return new Set(X.options.flatMap((Y) => Array.from(Y._zod.values)));
    return;
  }), $0(Q._zod, "pattern", () => {
    if (X.options.every((Y) => Y._zod.pattern)) {
      let Y = X.options.map(($) => $._zod.pattern);
      return new RegExp(`^(${Y.map(($) => s9($.source)).join("|")})$`);
    }
    return;
  }), Q._zod.parse = (Y, $) => {
    let J = false, W = [];
    for (let G of X.options) {
      let H = G._zod.run({ value: Y.value, issues: [] }, $);
      if (H instanceof Promise) W.push(H), J = true;
      else {
        if (H.issues.length === 0) return H;
        W.push(H);
      }
    }
    if (!J) return MW(W, Y, Q, $);
    return Promise.all(W).then((G) => {
      return MW(G, Y, Q, $);
    });
  };
});
var aX = D("$ZodDiscriminatedUnion", (Q, X) => {
  U8.init(Q, X);
  let Y = Q._zod.parse;
  $0(Q._zod, "propValues", () => {
    let J = {};
    for (let W of X.options) {
      let G = W._zod.propValues;
      if (!G || Object.keys(G).length === 0) throw Error(`Invalid discriminated union option at index "${X.options.indexOf(W)}"`);
      for (let [H, B] of Object.entries(G)) {
        if (!J[H]) J[H] = /* @__PURE__ */ new Set();
        for (let z of B) J[H].add(z);
      }
    }
    return J;
  });
  let $ = t9(() => {
    let J = X.options, W = /* @__PURE__ */ new Map();
    for (let G of J) {
      let H = G._zod.propValues[X.discriminator];
      if (!H || H.size === 0) throw Error(`Invalid discriminated union option at index "${X.options.indexOf(G)}"`);
      for (let B of H) {
        if (W.has(B)) throw Error(`Duplicate discriminator value "${String(B)}"`);
        W.set(B, G);
      }
    }
    return W;
  });
  Q._zod.parse = (J, W) => {
    let G = J.value;
    if (!K9(G)) return J.issues.push({ code: "invalid_type", expected: "object", input: G, inst: Q }), J;
    let H = $.value.get(G?.[X.discriminator]);
    if (H) return H._zod.run(J, W);
    if (X.unionFallback) return Y(J, W);
    return J.issues.push({ code: "invalid_union", errors: [], note: "No matching discriminator", input: G, path: [X.discriminator], inst: Q }), J;
  };
});
var sX = D("$ZodIntersection", (Q, X) => {
  e.init(Q, X), Q._zod.parse = (Y, $) => {
    let J = Y.value, W = X.left._zod.run({ value: J, issues: [] }, $), G = X.right._zod.run({ value: J, issues: [] }, $);
    if (W instanceof Promise || G instanceof Promise) return Promise.all([W, G]).then(([B, z]) => {
      return wW(Y, B, z);
    });
    return wW(Y, W, G);
  };
});
function bX(Q, X) {
  if (Q === X) return { valid: true, data: Q };
  if (Q instanceof Date && X instanceof Date && +Q === +X) return { valid: true, data: Q };
  if (V9(Q) && V9(X)) {
    let Y = Object.keys(X), $ = Object.keys(Q).filter((W) => Y.indexOf(W) !== -1), J = { ...Q, ...X };
    for (let W of $) {
      let G = bX(Q[W], X[W]);
      if (!G.valid) return { valid: false, mergeErrorPath: [W, ...G.mergeErrorPath] };
      J[W] = G.data;
    }
    return { valid: true, data: J };
  }
  if (Array.isArray(Q) && Array.isArray(X)) {
    if (Q.length !== X.length) return { valid: false, mergeErrorPath: [] };
    let Y = [];
    for (let $ = 0; $ < Q.length; $++) {
      let J = Q[$], W = X[$], G = bX(J, W);
      if (!G.valid) return { valid: false, mergeErrorPath: [$, ...G.mergeErrorPath] };
      Y.push(G.data);
    }
    return { valid: true, data: Y };
  }
  return { valid: false, mergeErrorPath: [] };
}
function wW(Q, X, Y) {
  if (X.issues.length) Q.issues.push(...X.issues);
  if (Y.issues.length) Q.issues.push(...Y.issues);
  if (w6(Q)) return Q;
  let $ = bX(X.value, Y.value);
  if (!$.valid) throw Error(`Unmergable intersection. Error path: ${JSON.stringify($.mergeErrorPath)}`);
  return Q.value = $.data, Q;
}
var eX = D("$ZodRecord", (Q, X) => {
  e.init(Q, X), Q._zod.parse = (Y, $) => {
    let J = Y.value;
    if (!V9(J)) return Y.issues.push({ expected: "record", code: "invalid_type", input: J, inst: Q }), Y;
    let W = [];
    if (X.keyType._zod.values) {
      let G = X.keyType._zod.values;
      Y.value = {};
      for (let B of G) if (typeof B === "string" || typeof B === "number" || typeof B === "symbol") {
        let z = X.valueType._zod.run({ value: J[B], issues: [] }, $);
        if (z instanceof Promise) W.push(z.then((K) => {
          if (K.issues.length) Y.issues.push(...j1(B, K.issues));
          Y.value[B] = K.value;
        }));
        else {
          if (z.issues.length) Y.issues.push(...j1(B, z.issues));
          Y.value[B] = z.value;
        }
      }
      let H;
      for (let B in J) if (!G.has(B)) H = H ?? [], H.push(B);
      if (H && H.length > 0) Y.issues.push({ code: "unrecognized_keys", input: J, inst: Q, keys: H });
    } else {
      Y.value = {};
      for (let G of Reflect.ownKeys(J)) {
        if (G === "__proto__") continue;
        let H = X.keyType._zod.run({ value: G, issues: [] }, $);
        if (H instanceof Promise) throw Error("Async schemas not supported in object keys currently");
        if (H.issues.length) {
          Y.issues.push({ origin: "record", code: "invalid_key", issues: H.issues.map((z) => G1(z, $, l0())), input: G, path: [G], inst: Q }), Y.value[H.value] = H.value;
          continue;
        }
        let B = X.valueType._zod.run({ value: J[G], issues: [] }, $);
        if (B instanceof Promise) W.push(B.then((z) => {
          if (z.issues.length) Y.issues.push(...j1(G, z.issues));
          Y.value[H.value] = z.value;
        }));
        else {
          if (B.issues.length) Y.issues.push(...j1(G, B.issues));
          Y.value[H.value] = B.value;
        }
      }
    }
    if (W.length) return Promise.all(W).then(() => Y);
    return Y;
  };
});
var QY = D("$ZodEnum", (Q, X) => {
  e.init(Q, X);
  let Y = r9(X.entries);
  Q._zod.values = new Set(Y), Q._zod.pattern = new RegExp(`^(${Y.filter(($) => BX.has(typeof $)).map(($) => typeof $ === "string" ? o1($) : $.toString()).join("|")})$`), Q._zod.parse = ($, J) => {
    let W = $.value;
    if (Q._zod.values.has(W)) return $;
    return $.issues.push({ code: "invalid_value", values: Y, input: W, inst: Q }), $;
  };
});
var XY = D("$ZodLiteral", (Q, X) => {
  e.init(Q, X), Q._zod.values = new Set(X.values), Q._zod.pattern = new RegExp(`^(${X.values.map((Y) => typeof Y === "string" ? o1(Y) : Y ? Y.toString() : String(Y)).join("|")})$`), Q._zod.parse = (Y, $) => {
    let J = Y.value;
    if (Q._zod.values.has(J)) return Y;
    return Y.issues.push({ code: "invalid_value", values: X.values, input: J, inst: Q }), Y;
  };
});
var YY = D("$ZodTransform", (Q, X) => {
  e.init(Q, X), Q._zod.parse = (Y, $) => {
    let J = X.transform(Y.value, Y);
    if ($.async) return (J instanceof Promise ? J : Promise.resolve(J)).then((G) => {
      return Y.value = G, Y;
    });
    if (J instanceof Promise) throw new n1();
    return Y.value = J, Y;
  };
});
var $Y = D("$ZodOptional", (Q, X) => {
  e.init(Q, X), Q._zod.optin = "optional", Q._zod.optout = "optional", $0(Q._zod, "values", () => {
    return X.innerType._zod.values ? /* @__PURE__ */ new Set([...X.innerType._zod.values, void 0]) : void 0;
  }), $0(Q._zod, "pattern", () => {
    let Y = X.innerType._zod.pattern;
    return Y ? new RegExp(`^(${s9(Y.source)})?$`) : void 0;
  }), Q._zod.parse = (Y, $) => {
    if (X.innerType._zod.optin === "optional") return X.innerType._zod.run(Y, $);
    if (Y.value === void 0) return Y;
    return X.innerType._zod.run(Y, $);
  };
});
var JY = D("$ZodNullable", (Q, X) => {
  e.init(Q, X), $0(Q._zod, "optin", () => X.innerType._zod.optin), $0(Q._zod, "optout", () => X.innerType._zod.optout), $0(Q._zod, "pattern", () => {
    let Y = X.innerType._zod.pattern;
    return Y ? new RegExp(`^(${s9(Y.source)}|null)$`) : void 0;
  }), $0(Q._zod, "values", () => {
    return X.innerType._zod.values ? /* @__PURE__ */ new Set([...X.innerType._zod.values, null]) : void 0;
  }), Q._zod.parse = (Y, $) => {
    if (Y.value === null) return Y;
    return X.innerType._zod.run(Y, $);
  };
});
var WY = D("$ZodDefault", (Q, X) => {
  e.init(Q, X), Q._zod.optin = "optional", $0(Q._zod, "values", () => X.innerType._zod.values), Q._zod.parse = (Y, $) => {
    if (Y.value === void 0) return Y.value = X.defaultValue, Y;
    let J = X.innerType._zod.run(Y, $);
    if (J instanceof Promise) return J.then((W) => AW(W, X));
    return AW(J, X);
  };
});
function AW(Q, X) {
  if (Q.value === void 0) Q.value = X.defaultValue;
  return Q;
}
var GY = D("$ZodPrefault", (Q, X) => {
  e.init(Q, X), Q._zod.optin = "optional", $0(Q._zod, "values", () => X.innerType._zod.values), Q._zod.parse = (Y, $) => {
    if (Y.value === void 0) Y.value = X.defaultValue;
    return X.innerType._zod.run(Y, $);
  };
});
var HY = D("$ZodNonOptional", (Q, X) => {
  e.init(Q, X), $0(Q._zod, "values", () => {
    let Y = X.innerType._zod.values;
    return Y ? new Set([...Y].filter(($) => $ !== void 0)) : void 0;
  }), Q._zod.parse = (Y, $) => {
    let J = X.innerType._zod.run(Y, $);
    if (J instanceof Promise) return J.then((W) => jW(W, Q));
    return jW(J, Q);
  };
});
function jW(Q, X) {
  if (!Q.issues.length && Q.value === void 0) Q.issues.push({ code: "invalid_type", expected: "nonoptional", input: Q.value, inst: X });
  return Q;
}
var BY = D("$ZodCatch", (Q, X) => {
  e.init(Q, X), Q._zod.optin = "optional", $0(Q._zod, "optout", () => X.innerType._zod.optout), $0(Q._zod, "values", () => X.innerType._zod.values), Q._zod.parse = (Y, $) => {
    let J = X.innerType._zod.run(Y, $);
    if (J instanceof Promise) return J.then((W) => {
      if (Y.value = W.value, W.issues.length) Y.value = X.catchValue({ ...Y, error: { issues: W.issues.map((G) => G1(G, $, l0())) }, input: Y.value }), Y.issues = [];
      return Y;
    });
    if (Y.value = J.value, J.issues.length) Y.value = X.catchValue({ ...Y, error: { issues: J.issues.map((W) => G1(W, $, l0())) }, input: Y.value }), Y.issues = [];
    return Y;
  };
});
var zY = D("$ZodPipe", (Q, X) => {
  e.init(Q, X), $0(Q._zod, "values", () => X.in._zod.values), $0(Q._zod, "optin", () => X.in._zod.optin), $0(Q._zod, "optout", () => X.out._zod.optout), Q._zod.parse = (Y, $) => {
    let J = X.in._zod.run(Y, $);
    if (J instanceof Promise) return J.then((W) => RW(W, X, $));
    return RW(J, X, $);
  };
});
function RW(Q, X, Y) {
  if (w6(Q)) return Q;
  return X.out._zod.run({ value: Q.value, issues: Q.issues }, Y);
}
var KY = D("$ZodReadonly", (Q, X) => {
  e.init(Q, X), $0(Q._zod, "propValues", () => X.innerType._zod.propValues), $0(Q._zod, "values", () => X.innerType._zod.values), $0(Q._zod, "optin", () => X.innerType._zod.optin), $0(Q._zod, "optout", () => X.innerType._zod.optout), Q._zod.parse = (Y, $) => {
    let J = X.innerType._zod.run(Y, $);
    if (J instanceof Promise) return J.then(IW);
    return IW(J);
  };
});
function IW(Q) {
  return Q.value = Object.freeze(Q.value), Q;
}
var VY = D("$ZodCustom", (Q, X) => {
  j0.init(Q, X), e.init(Q, X), Q._zod.parse = (Y, $) => {
    return Y;
  }, Q._zod.check = (Y) => {
    let $ = Y.value, J = X.fn($);
    if (J instanceof Promise) return J.then((W) => bW(W, Y, $, Q));
    bW(J, Y, $, Q);
    return;
  };
});
function bW(Q, X, Y, $) {
  if (!Q) {
    let J = { code: "custom", input: Y, inst: $, path: [...$._zod.def.path ?? []], continue: !$._zod.def.abort };
    if ($._zod.def.params) J.params = $._zod.def.params;
    X.issues.push(VX(J));
  }
}
var xF = (Q) => {
  let X = typeof Q;
  switch (X) {
    case "number":
      return Number.isNaN(Q) ? "NaN" : "number";
    case "object": {
      if (Array.isArray(Q)) return "array";
      if (Q === null) return "null";
      if (Object.getPrototypeOf(Q) !== Object.prototype && Q.constructor) return Q.constructor.name;
    }
  }
  return X;
};
var yF = () => {
  let Q = { string: { unit: "characters", verb: "to have" }, file: { unit: "bytes", verb: "to have" }, array: { unit: "items", verb: "to have" }, set: { unit: "items", verb: "to have" } };
  function X($) {
    return Q[$] ?? null;
  }
  let Y = { regex: "input", email: "email address", url: "URL", emoji: "emoji", uuid: "UUID", uuidv4: "UUIDv4", uuidv6: "UUIDv6", nanoid: "nanoid", guid: "GUID", cuid: "cuid", cuid2: "cuid2", ulid: "ULID", xid: "XID", ksuid: "KSUID", datetime: "ISO datetime", date: "ISO date", time: "ISO time", duration: "ISO duration", ipv4: "IPv4 address", ipv6: "IPv6 address", cidrv4: "IPv4 range", cidrv6: "IPv6 range", base64: "base64-encoded string", base64url: "base64url-encoded string", json_string: "JSON string", e164: "E.164 number", jwt: "JWT", template_literal: "input" };
  return ($) => {
    switch ($.code) {
      case "invalid_type":
        return `Invalid input: expected ${$.expected}, received ${xF($.input)}`;
      case "invalid_value":
        if ($.values.length === 1) return `Invalid input: expected ${H8($.values[0])}`;
        return `Invalid option: expected one of ${W8($.values, "|")}`;
      case "too_big": {
        let J = $.inclusive ? "<=" : "<", W = X($.origin);
        if (W) return `Too big: expected ${$.origin ?? "value"} to have ${J}${$.maximum.toString()} ${W.unit ?? "elements"}`;
        return `Too big: expected ${$.origin ?? "value"} to be ${J}${$.maximum.toString()}`;
      }
      case "too_small": {
        let J = $.inclusive ? ">=" : ">", W = X($.origin);
        if (W) return `Too small: expected ${$.origin} to have ${J}${$.minimum.toString()} ${W.unit}`;
        return `Too small: expected ${$.origin} to be ${J}${$.minimum.toString()}`;
      }
      case "invalid_format": {
        let J = $;
        if (J.format === "starts_with") return `Invalid string: must start with "${J.prefix}"`;
        if (J.format === "ends_with") return `Invalid string: must end with "${J.suffix}"`;
        if (J.format === "includes") return `Invalid string: must include "${J.includes}"`;
        if (J.format === "regex") return `Invalid string: must match pattern ${J.pattern}`;
        return `Invalid ${Y[J.format] ?? $.format}`;
      }
      case "not_multiple_of":
        return `Invalid number: must be a multiple of ${$.divisor}`;
      case "unrecognized_keys":
        return `Unrecognized key${$.keys.length > 1 ? "s" : ""}: ${W8($.keys, ", ")}`;
      case "invalid_key":
        return `Invalid key in ${$.origin}`;
      case "invalid_union":
        return "Invalid input";
      case "invalid_element":
        return `Invalid value in ${$.origin}`;
      default:
        return "Invalid input";
    }
  };
};
function qY() {
  return { localeError: yF() };
}
var L8 = class {
  constructor() {
    this._map = /* @__PURE__ */ new WeakMap(), this._idmap = /* @__PURE__ */ new Map();
  }
  add(Q, ...X) {
    let Y = X[0];
    if (this._map.set(Q, Y), Y && typeof Y === "object" && "id" in Y) {
      if (this._idmap.has(Y.id)) throw Error(`ID ${Y.id} already exists in the registry`);
      this._idmap.set(Y.id, Q);
    }
    return this;
  }
  remove(Q) {
    return this._map.delete(Q), this;
  }
  get(Q) {
    let X = Q._zod.parent;
    if (X) {
      let Y = { ...this.get(X) ?? {} };
      return delete Y.id, { ...Y, ...this._map.get(Q) };
    }
    return this._map.get(Q);
  }
  has(Q) {
    return this._map.has(Q);
  }
};
function _W() {
  return new L8();
}
var r1 = _W();
function UY(Q, X) {
  return new Q({ type: "string", ...y(X) });
}
function LY(Q, X) {
  return new Q({ type: "string", format: "email", check: "string_format", abort: false, ...y(X) });
}
function F8(Q, X) {
  return new Q({ type: "string", format: "guid", check: "string_format", abort: false, ...y(X) });
}
function FY(Q, X) {
  return new Q({ type: "string", format: "uuid", check: "string_format", abort: false, ...y(X) });
}
function NY(Q, X) {
  return new Q({ type: "string", format: "uuid", check: "string_format", abort: false, version: "v4", ...y(X) });
}
function OY(Q, X) {
  return new Q({ type: "string", format: "uuid", check: "string_format", abort: false, version: "v6", ...y(X) });
}
function DY(Q, X) {
  return new Q({ type: "string", format: "uuid", check: "string_format", abort: false, version: "v7", ...y(X) });
}
function MY(Q, X) {
  return new Q({ type: "string", format: "url", check: "string_format", abort: false, ...y(X) });
}
function wY(Q, X) {
  return new Q({ type: "string", format: "emoji", check: "string_format", abort: false, ...y(X) });
}
function AY(Q, X) {
  return new Q({ type: "string", format: "nanoid", check: "string_format", abort: false, ...y(X) });
}
function jY(Q, X) {
  return new Q({ type: "string", format: "cuid", check: "string_format", abort: false, ...y(X) });
}
function RY(Q, X) {
  return new Q({ type: "string", format: "cuid2", check: "string_format", abort: false, ...y(X) });
}
function IY(Q, X) {
  return new Q({ type: "string", format: "ulid", check: "string_format", abort: false, ...y(X) });
}
function bY(Q, X) {
  return new Q({ type: "string", format: "xid", check: "string_format", abort: false, ...y(X) });
}
function EY(Q, X) {
  return new Q({ type: "string", format: "ksuid", check: "string_format", abort: false, ...y(X) });
}
function PY(Q, X) {
  return new Q({ type: "string", format: "ipv4", check: "string_format", abort: false, ...y(X) });
}
function ZY(Q, X) {
  return new Q({ type: "string", format: "ipv6", check: "string_format", abort: false, ...y(X) });
}
function CY(Q, X) {
  return new Q({ type: "string", format: "cidrv4", check: "string_format", abort: false, ...y(X) });
}
function SY(Q, X) {
  return new Q({ type: "string", format: "cidrv6", check: "string_format", abort: false, ...y(X) });
}
function _Y(Q, X) {
  return new Q({ type: "string", format: "base64", check: "string_format", abort: false, ...y(X) });
}
function kY(Q, X) {
  return new Q({ type: "string", format: "base64url", check: "string_format", abort: false, ...y(X) });
}
function vY(Q, X) {
  return new Q({ type: "string", format: "e164", check: "string_format", abort: false, ...y(X) });
}
function TY(Q, X) {
  return new Q({ type: "string", format: "jwt", check: "string_format", abort: false, ...y(X) });
}
function kW(Q, X) {
  return new Q({ type: "string", format: "datetime", check: "string_format", offset: false, local: false, precision: null, ...y(X) });
}
function vW(Q, X) {
  return new Q({ type: "string", format: "date", check: "string_format", ...y(X) });
}
function TW(Q, X) {
  return new Q({ type: "string", format: "time", check: "string_format", precision: null, ...y(X) });
}
function xW(Q, X) {
  return new Q({ type: "string", format: "duration", check: "string_format", ...y(X) });
}
function xY(Q, X) {
  return new Q({ type: "number", checks: [], ...y(X) });
}
function yY(Q, X) {
  return new Q({ type: "number", check: "number_format", abort: false, format: "safeint", ...y(X) });
}
function gY(Q, X) {
  return new Q({ type: "boolean", ...y(X) });
}
function hY(Q, X) {
  return new Q({ type: "null", ...y(X) });
}
function fY(Q) {
  return new Q({ type: "unknown" });
}
function uY(Q, X) {
  return new Q({ type: "never", ...y(X) });
}
function N8(Q, X) {
  return new jX({ check: "less_than", ...y(X), value: Q, inclusive: false });
}
function $4(Q, X) {
  return new jX({ check: "less_than", ...y(X), value: Q, inclusive: true });
}
function O8(Q, X) {
  return new RX({ check: "greater_than", ...y(X), value: Q, inclusive: false });
}
function J4(Q, X) {
  return new RX({ check: "greater_than", ...y(X), value: Q, inclusive: true });
}
function D8(Q, X) {
  return new $W({ check: "multiple_of", ...y(X), value: Q });
}
function M8(Q, X) {
  return new WW({ check: "max_length", ...y(X), maximum: Q });
}
function q9(Q, X) {
  return new GW({ check: "min_length", ...y(X), minimum: Q });
}
function w8(Q, X) {
  return new HW({ check: "length_equals", ...y(X), length: Q });
}
function mY(Q, X) {
  return new BW({ check: "string_format", format: "regex", ...y(X), pattern: Q });
}
function lY(Q) {
  return new zW({ check: "string_format", format: "lowercase", ...y(Q) });
}
function cY(Q) {
  return new KW({ check: "string_format", format: "uppercase", ...y(Q) });
}
function pY(Q, X) {
  return new VW({ check: "string_format", format: "includes", ...y(X), includes: Q });
}
function dY(Q, X) {
  return new qW({ check: "string_format", format: "starts_with", ...y(X), prefix: Q });
}
function iY(Q, X) {
  return new UW({ check: "string_format", format: "ends_with", ...y(X), suffix: Q });
}
function R6(Q) {
  return new LW({ check: "overwrite", tx: Q });
}
function nY(Q) {
  return R6((X) => X.normalize(Q));
}
function oY() {
  return R6((Q) => Q.trim());
}
function rY() {
  return R6((Q) => Q.toLowerCase());
}
function tY() {
  return R6((Q) => Q.toUpperCase());
}
function yW(Q, X, Y) {
  return new Q({ type: "array", element: X, ...y(Y) });
}
function aY(Q, X, Y) {
  let $ = y(Y);
  return $.abort ?? ($.abort = true), new Q({ type: "custom", check: "custom", fn: X, ...$ });
}
function sY(Q, X, Y) {
  return new Q({ type: "custom", check: "custom", fn: X, ...y(Y) });
}
var MN = D("ZodMiniType", (Q, X) => {
  if (!Q._zod) throw Error("Uninitialized schema in ZodMiniType.");
  e.init(Q, X), Q.def = X, Q.parse = (Y, $) => FX(Q, Y, $, { callee: Q.parse }), Q.safeParse = (Y, $) => A6(Q, Y, $), Q.parseAsync = async (Y, $) => OX(Q, Y, $, { callee: Q.parseAsync }), Q.safeParseAsync = async (Y, $) => j6(Q, Y, $), Q.check = (...Y) => {
    return Q.clone({ ...X, checks: [...X.checks ?? [], ...Y.map(($) => typeof $ === "function" ? { _zod: { check: $, def: { check: "custom" }, onattach: [] } } : $)] });
  }, Q.clone = (Y, $) => c0(Q, Y, $), Q.brand = () => Q, Q.register = (Y, $) => {
    return Y.add(Q, $), Q;
  };
});
var wN = D("ZodMiniObject", (Q, X) => {
  q8.init(Q, X), MN.init(Q, X), i.defineLazy(Q, "shape", () => X.shape);
});
var W4 = {};
gQ(W4, { time: () => J$, duration: () => W$, datetime: () => Y$, date: () => $$, ZodISOTime: () => lW, ZodISODuration: () => cW, ZodISODateTime: () => uW, ZodISODate: () => mW });
var uW = D("ZodISODateTime", (Q, X) => {
  EW.init(Q, X), z0.init(Q, X);
});
function Y$(Q) {
  return kW(uW, Q);
}
var mW = D("ZodISODate", (Q, X) => {
  PW.init(Q, X), z0.init(Q, X);
});
function $$(Q) {
  return vW(mW, Q);
}
var lW = D("ZodISOTime", (Q, X) => {
  ZW.init(Q, X), z0.init(Q, X);
});
function J$(Q) {
  return TW(lW, Q);
}
var cW = D("ZodISODuration", (Q, X) => {
  CW.init(Q, X), z0.init(Q, X);
});
function W$(Q) {
  return xW(cW, Q);
}
var pW = (Q, X) => {
  B8.init(Q, X), Q.name = "ZodError", Object.defineProperties(Q, { format: { value: (Y) => UX(Q, Y) }, flatten: { value: (Y) => qX(Q, Y) }, addIssue: { value: (Y) => Q.issues.push(Y) }, addIssues: { value: (Y) => Q.issues.push(...Y) }, isEmpty: { get() {
    return Q.issues.length === 0;
  } } });
};
var d_ = D("ZodError", pW);
var G4 = D("ZodError", pW, { Parent: Error });
var dW = LX(G4);
var iW = NX(G4);
var nW = DX(G4);
var oW = MX(G4);
var N0 = D("ZodType", (Q, X) => {
  return e.init(Q, X), Q.def = X, Object.defineProperty(Q, "_def", { value: X }), Q.check = (...Y) => {
    return Q.clone({ ...X, checks: [...X.checks ?? [], ...Y.map(($) => typeof $ === "function" ? { _zod: { check: $, def: { check: "custom" }, onattach: [] } } : $)] });
  }, Q.clone = (Y, $) => c0(Q, Y, $), Q.brand = () => Q, Q.register = (Y, $) => {
    return Y.add(Q, $), Q;
  }, Q.parse = (Y, $) => dW(Q, Y, $, { callee: Q.parse }), Q.safeParse = (Y, $) => nW(Q, Y, $), Q.parseAsync = async (Y, $) => iW(Q, Y, $, { callee: Q.parseAsync }), Q.safeParseAsync = async (Y, $) => oW(Q, Y, $), Q.spa = Q.safeParseAsync, Q.refine = (Y, $) => Q.check(OO(Y, $)), Q.superRefine = (Y) => Q.check(DO(Y)), Q.overwrite = (Y) => Q.check(R6(Y)), Q.optional = () => F0(Q), Q.nullable = () => aW(Q), Q.nullish = () => F0(aW(Q)), Q.nonoptional = (Y) => KO(Q, Y), Q.array = () => n(Q), Q.or = (Y) => W0([Q, Y]), Q.and = (Y) => b8(Q, Y), Q.transform = (Y) => H$(Q, YG(Y)), Q.default = (Y) => HO(Q, Y), Q.prefault = (Y) => zO(Q, Y), Q.catch = (Y) => qO(Q, Y), Q.pipe = (Y) => H$(Q, Y), Q.readonly = () => FO(Q), Q.describe = (Y) => {
    let $ = Q.clone();
    return r1.add($, { description: Y }), $;
  }, Object.defineProperty(Q, "description", { get() {
    return r1.get(Q)?.description;
  }, configurable: true }), Q.meta = (...Y) => {
    if (Y.length === 0) return r1.get(Q);
    let $ = Q.clone();
    return r1.add($, Y[0]), $;
  }, Q.isOptional = () => Q.safeParse(void 0).success, Q.isNullable = () => Q.safeParse(null).success, Q;
});
var sW = D("_ZodString", (Q, X) => {
  Y4.init(Q, X), N0.init(Q, X);
  let Y = Q._zod.bag;
  Q.format = Y.format ?? null, Q.minLength = Y.minimum ?? null, Q.maxLength = Y.maximum ?? null, Q.regex = (...$) => Q.check(mY(...$)), Q.includes = (...$) => Q.check(pY(...$)), Q.startsWith = (...$) => Q.check(dY(...$)), Q.endsWith = (...$) => Q.check(iY(...$)), Q.min = (...$) => Q.check(q9(...$)), Q.max = (...$) => Q.check(M8(...$)), Q.length = (...$) => Q.check(w8(...$)), Q.nonempty = (...$) => Q.check(q9(1, ...$)), Q.lowercase = ($) => Q.check(lY($)), Q.uppercase = ($) => Q.check(cY($)), Q.trim = () => Q.check(oY()), Q.normalize = (...$) => Q.check(nY(...$)), Q.toLowerCase = () => Q.check(rY()), Q.toUpperCase = () => Q.check(tY());
});
var CN = D("ZodString", (Q, X) => {
  Y4.init(Q, X), sW.init(Q, X), Q.email = (Y) => Q.check(LY(SN, Y)), Q.url = (Y) => Q.check(MY(_N, Y)), Q.jwt = (Y) => Q.check(TY(iN, Y)), Q.emoji = (Y) => Q.check(wY(kN, Y)), Q.guid = (Y) => Q.check(F8(rW, Y)), Q.uuid = (Y) => Q.check(FY(I8, Y)), Q.uuidv4 = (Y) => Q.check(NY(I8, Y)), Q.uuidv6 = (Y) => Q.check(OY(I8, Y)), Q.uuidv7 = (Y) => Q.check(DY(I8, Y)), Q.nanoid = (Y) => Q.check(AY(vN, Y)), Q.guid = (Y) => Q.check(F8(rW, Y)), Q.cuid = (Y) => Q.check(jY(TN, Y)), Q.cuid2 = (Y) => Q.check(RY(xN, Y)), Q.ulid = (Y) => Q.check(IY(yN, Y)), Q.base64 = (Y) => Q.check(_Y(cN, Y)), Q.base64url = (Y) => Q.check(kY(pN, Y)), Q.xid = (Y) => Q.check(bY(gN, Y)), Q.ksuid = (Y) => Q.check(EY(hN, Y)), Q.ipv4 = (Y) => Q.check(PY(fN, Y)), Q.ipv6 = (Y) => Q.check(ZY(uN, Y)), Q.cidrv4 = (Y) => Q.check(CY(mN, Y)), Q.cidrv6 = (Y) => Q.check(SY(lN, Y)), Q.e164 = (Y) => Q.check(vY(dN, Y)), Q.datetime = (Y) => Q.check(Y$(Y)), Q.date = (Y) => Q.check($$(Y)), Q.time = (Y) => Q.check(J$(Y)), Q.duration = (Y) => Q.check(W$(Y));
});
function N(Q) {
  return UY(CN, Q);
}
var z0 = D("ZodStringFormat", (Q, X) => {
  J0.init(Q, X), sW.init(Q, X);
});
var SN = D("ZodEmail", (Q, X) => {
  ZX.init(Q, X), z0.init(Q, X);
});
var rW = D("ZodGUID", (Q, X) => {
  EX.init(Q, X), z0.init(Q, X);
});
var I8 = D("ZodUUID", (Q, X) => {
  PX.init(Q, X), z0.init(Q, X);
});
var _N = D("ZodURL", (Q, X) => {
  CX.init(Q, X), z0.init(Q, X);
});
var kN = D("ZodEmoji", (Q, X) => {
  SX.init(Q, X), z0.init(Q, X);
});
var vN = D("ZodNanoID", (Q, X) => {
  _X.init(Q, X), z0.init(Q, X);
});
var TN = D("ZodCUID", (Q, X) => {
  kX.init(Q, X), z0.init(Q, X);
});
var xN = D("ZodCUID2", (Q, X) => {
  vX.init(Q, X), z0.init(Q, X);
});
var yN = D("ZodULID", (Q, X) => {
  TX.init(Q, X), z0.init(Q, X);
});
var gN = D("ZodXID", (Q, X) => {
  xX.init(Q, X), z0.init(Q, X);
});
var hN = D("ZodKSUID", (Q, X) => {
  yX.init(Q, X), z0.init(Q, X);
});
var fN = D("ZodIPv4", (Q, X) => {
  gX.init(Q, X), z0.init(Q, X);
});
var uN = D("ZodIPv6", (Q, X) => {
  hX.init(Q, X), z0.init(Q, X);
});
var mN = D("ZodCIDRv4", (Q, X) => {
  fX.init(Q, X), z0.init(Q, X);
});
var lN = D("ZodCIDRv6", (Q, X) => {
  uX.init(Q, X), z0.init(Q, X);
});
var cN = D("ZodBase64", (Q, X) => {
  mX.init(Q, X), z0.init(Q, X);
});
var pN = D("ZodBase64URL", (Q, X) => {
  lX.init(Q, X), z0.init(Q, X);
});
var dN = D("ZodE164", (Q, X) => {
  cX.init(Q, X), z0.init(Q, X);
});
var iN = D("ZodJWT", (Q, X) => {
  pX.init(Q, X), z0.init(Q, X);
});
var eW = D("ZodNumber", (Q, X) => {
  V8.init(Q, X), N0.init(Q, X), Q.gt = ($, J) => Q.check(O8($, J)), Q.gte = ($, J) => Q.check(J4($, J)), Q.min = ($, J) => Q.check(J4($, J)), Q.lt = ($, J) => Q.check(N8($, J)), Q.lte = ($, J) => Q.check($4($, J)), Q.max = ($, J) => Q.check($4($, J)), Q.int = ($) => Q.check(tW($)), Q.safe = ($) => Q.check(tW($)), Q.positive = ($) => Q.check(O8(0, $)), Q.nonnegative = ($) => Q.check(J4(0, $)), Q.negative = ($) => Q.check(N8(0, $)), Q.nonpositive = ($) => Q.check($4(0, $)), Q.multipleOf = ($, J) => Q.check(D8($, J)), Q.step = ($, J) => Q.check(D8($, J)), Q.finite = () => Q;
  let Y = Q._zod.bag;
  Q.minValue = Math.max(Y.minimum ?? Number.NEGATIVE_INFINITY, Y.exclusiveMinimum ?? Number.NEGATIVE_INFINITY) ?? null, Q.maxValue = Math.min(Y.maximum ?? Number.POSITIVE_INFINITY, Y.exclusiveMaximum ?? Number.POSITIVE_INFINITY) ?? null, Q.isInt = (Y.format ?? "").includes("int") || Number.isSafeInteger(Y.multipleOf ?? 0.5), Q.isFinite = true, Q.format = Y.format ?? null;
});
function s(Q) {
  return xY(eW, Q);
}
var nN = D("ZodNumberFormat", (Q, X) => {
  dX.init(Q, X), eW.init(Q, X);
});
function tW(Q) {
  return yY(nN, Q);
}
var oN = D("ZodBoolean", (Q, X) => {
  iX.init(Q, X), N0.init(Q, X);
});
function w0(Q) {
  return gY(oN, Q);
}
var rN = D("ZodNull", (Q, X) => {
  nX.init(Q, X), N0.init(Q, X);
});
function B$(Q) {
  return hY(rN, Q);
}
var tN = D("ZodUnknown", (Q, X) => {
  oX.init(Q, X), N0.init(Q, X);
});
function K0() {
  return fY(tN);
}
var aN = D("ZodNever", (Q, X) => {
  rX.init(Q, X), N0.init(Q, X);
});
function sN(Q) {
  return uY(aN, Q);
}
var eN = D("ZodArray", (Q, X) => {
  tX.init(Q, X), N0.init(Q, X), Q.element = X.element, Q.min = (Y, $) => Q.check(q9(Y, $)), Q.nonempty = (Y) => Q.check(q9(1, Y)), Q.max = (Y, $) => Q.check(M8(Y, $)), Q.length = (Y, $) => Q.check(w8(Y, $)), Q.unwrap = () => Q.element;
});
function n(Q, X) {
  return yW(eN, Q, X);
}
var QG = D("ZodObject", (Q, X) => {
  q8.init(Q, X), N0.init(Q, X), i.defineLazy(Q, "shape", () => X.shape), Q.keyof = () => y0(Object.keys(Q._zod.def.shape)), Q.catchall = (Y) => Q.clone({ ...Q._zod.def, catchall: Y }), Q.passthrough = () => Q.clone({ ...Q._zod.def, catchall: K0() }), Q.loose = () => Q.clone({ ...Q._zod.def, catchall: K0() }), Q.strict = () => Q.clone({ ...Q._zod.def, catchall: sN() }), Q.strip = () => Q.clone({ ...Q._zod.def, catchall: void 0 }), Q.extend = (Y) => {
    return i.extend(Q, Y);
  }, Q.merge = (Y) => i.merge(Q, Y), Q.pick = (Y) => i.pick(Q, Y), Q.omit = (Y) => i.omit(Q, Y), Q.partial = (...Y) => i.partial($G, Q, Y[0]), Q.required = (...Y) => i.required(JG, Q, Y[0]);
});
function P(Q, X) {
  let Y = { type: "object", get shape() {
    return i.assignProp(this, "shape", { ...Q }), this.shape;
  }, ...i.normalizeParams(X) };
  return new QG(Y);
}
function S0(Q, X) {
  return new QG({ type: "object", get shape() {
    return i.assignProp(this, "shape", { ...Q }), this.shape;
  }, catchall: K0(), ...i.normalizeParams(X) });
}
var XG = D("ZodUnion", (Q, X) => {
  U8.init(Q, X), N0.init(Q, X), Q.options = X.options;
});
function W0(Q, X) {
  return new XG({ type: "union", options: Q, ...i.normalizeParams(X) });
}
var QO = D("ZodDiscriminatedUnion", (Q, X) => {
  XG.init(Q, X), aX.init(Q, X);
});
function z$(Q, X, Y) {
  return new QO({ type: "union", options: X, discriminator: Q, ...i.normalizeParams(Y) });
}
var XO = D("ZodIntersection", (Q, X) => {
  sX.init(Q, X), N0.init(Q, X);
});
function b8(Q, X) {
  return new XO({ type: "intersection", left: Q, right: X });
}
var YO = D("ZodRecord", (Q, X) => {
  eX.init(Q, X), N0.init(Q, X), Q.keyType = X.keyType, Q.valueType = X.valueType;
});
function V0(Q, X, Y) {
  return new YO({ type: "record", keyType: Q, valueType: X, ...i.normalizeParams(Y) });
}
var G$ = D("ZodEnum", (Q, X) => {
  QY.init(Q, X), N0.init(Q, X), Q.enum = X.entries, Q.options = Object.values(X.entries);
  let Y = new Set(Object.keys(X.entries));
  Q.extract = ($, J) => {
    let W = {};
    for (let G of $) if (Y.has(G)) W[G] = X.entries[G];
    else throw Error(`Key ${G} not found in enum`);
    return new G$({ ...X, checks: [], ...i.normalizeParams(J), entries: W });
  }, Q.exclude = ($, J) => {
    let W = { ...X.entries };
    for (let G of $) if (Y.has(G)) delete W[G];
    else throw Error(`Key ${G} not found in enum`);
    return new G$({ ...X, checks: [], ...i.normalizeParams(J), entries: W });
  };
});
function y0(Q, X) {
  let Y = Array.isArray(Q) ? Object.fromEntries(Q.map(($) => [$, $])) : Q;
  return new G$({ type: "enum", entries: Y, ...i.normalizeParams(X) });
}
var $O = D("ZodLiteral", (Q, X) => {
  XY.init(Q, X), N0.init(Q, X), Q.values = new Set(X.values), Object.defineProperty(Q, "value", { get() {
    if (X.values.length > 1) throw Error("This schema contains multiple valid literal values. Use `.values` instead.");
    return X.values[0];
  } });
});
function k(Q, X) {
  return new $O({ type: "literal", values: Array.isArray(Q) ? Q : [Q], ...i.normalizeParams(X) });
}
var JO = D("ZodTransform", (Q, X) => {
  YY.init(Q, X), N0.init(Q, X), Q._zod.parse = (Y, $) => {
    Y.addIssue = (W) => {
      if (typeof W === "string") Y.issues.push(i.issue(W, Y.value, X));
      else {
        let G = W;
        if (G.fatal) G.continue = false;
        G.code ?? (G.code = "custom"), G.input ?? (G.input = Y.value), G.inst ?? (G.inst = Q), G.continue ?? (G.continue = true), Y.issues.push(i.issue(G));
      }
    };
    let J = X.transform(Y.value, Y);
    if (J instanceof Promise) return J.then((W) => {
      return Y.value = W, Y;
    });
    return Y.value = J, Y;
  };
});
function YG(Q) {
  return new JO({ type: "transform", transform: Q });
}
var $G = D("ZodOptional", (Q, X) => {
  $Y.init(Q, X), N0.init(Q, X), Q.unwrap = () => Q._zod.def.innerType;
});
function F0(Q) {
  return new $G({ type: "optional", innerType: Q });
}
var WO = D("ZodNullable", (Q, X) => {
  JY.init(Q, X), N0.init(Q, X), Q.unwrap = () => Q._zod.def.innerType;
});
function aW(Q) {
  return new WO({ type: "nullable", innerType: Q });
}
var GO = D("ZodDefault", (Q, X) => {
  WY.init(Q, X), N0.init(Q, X), Q.unwrap = () => Q._zod.def.innerType, Q.removeDefault = Q.unwrap;
});
function HO(Q, X) {
  return new GO({ type: "default", innerType: Q, get defaultValue() {
    return typeof X === "function" ? X() : X;
  } });
}
var BO = D("ZodPrefault", (Q, X) => {
  GY.init(Q, X), N0.init(Q, X), Q.unwrap = () => Q._zod.def.innerType;
});
function zO(Q, X) {
  return new BO({ type: "prefault", innerType: Q, get defaultValue() {
    return typeof X === "function" ? X() : X;
  } });
}
var JG = D("ZodNonOptional", (Q, X) => {
  HY.init(Q, X), N0.init(Q, X), Q.unwrap = () => Q._zod.def.innerType;
});
function KO(Q, X) {
  return new JG({ type: "nonoptional", innerType: Q, ...i.normalizeParams(X) });
}
var VO = D("ZodCatch", (Q, X) => {
  BY.init(Q, X), N0.init(Q, X), Q.unwrap = () => Q._zod.def.innerType, Q.removeCatch = Q.unwrap;
});
function qO(Q, X) {
  return new VO({ type: "catch", innerType: Q, catchValue: typeof X === "function" ? X : () => X });
}
var UO = D("ZodPipe", (Q, X) => {
  zY.init(Q, X), N0.init(Q, X), Q.in = X.in, Q.out = X.out;
});
function H$(Q, X) {
  return new UO({ type: "pipe", in: Q, out: X });
}
var LO = D("ZodReadonly", (Q, X) => {
  KY.init(Q, X), N0.init(Q, X);
});
function FO(Q) {
  return new LO({ type: "readonly", innerType: Q });
}
var WG = D("ZodCustom", (Q, X) => {
  VY.init(Q, X), N0.init(Q, X);
});
function NO(Q, X) {
  let Y = new j0({ check: "custom", ...i.normalizeParams(X) });
  return Y._zod.check = Q, Y;
}
function GG(Q, X) {
  return aY(WG, Q ?? (() => true), X);
}
function OO(Q, X = {}) {
  return sY(WG, Q, X);
}
function DO(Q, X) {
  let Y = NO(($) => {
    return $.addIssue = (J) => {
      if (typeof J === "string") $.issues.push(i.issue(J, $.value, Y._zod.def));
      else {
        let W = J;
        if (W.fatal) W.continue = false;
        W.code ?? (W.code = "custom"), W.input ?? (W.input = $.value), W.inst ?? (W.inst = Y), W.continue ?? (W.continue = !Y._zod.def.abort), $.issues.push(i.issue(W));
      }
    }, Q($.value, $);
  }, X);
  return Y;
}
function K$(Q, X) {
  return H$(YG(Q), X);
}
l0(qY());
var s1 = "io.modelcontextprotocol/related-task";
var P8 = "2.0";
var R0 = GG((Q) => Q !== null && (typeof Q === "object" || typeof Q === "function"));
var BG = W0([N(), s().int()]);
var zG = N();
var Wk = S0({ ttl: W0([s(), B$()]).optional(), pollInterval: s().optional() });
var MO = P({ ttl: s().optional() });
var wO = P({ taskId: N() });
var q$ = S0({ progressToken: BG.optional(), [s1]: wO.optional() });
var p0 = P({ _meta: q$.optional() });
var H4 = p0.extend({ task: MO.optional() });
var I0 = P({ method: N(), params: p0.loose().optional() });
var a0 = P({ _meta: q$.optional() });
var s0 = P({ method: N(), params: a0.loose().optional() });
var b0 = S0({ _meta: q$.optional() });
var Z8 = W0([N(), s().int()]);
var VG = P({ jsonrpc: k(P8), id: Z8, ...I0.shape }).strict();
var qG = P({ jsonrpc: k(P8), ...s0.shape }).strict();
var L$ = P({ jsonrpc: k(P8), id: Z8, result: b0 }).strict();
var T;
(function(Q) {
  Q[Q.ConnectionClosed = -32e3] = "ConnectionClosed", Q[Q.RequestTimeout = -32001] = "RequestTimeout", Q[Q.ParseError = -32700] = "ParseError", Q[Q.InvalidRequest = -32600] = "InvalidRequest", Q[Q.MethodNotFound = -32601] = "MethodNotFound", Q[Q.InvalidParams = -32602] = "InvalidParams", Q[Q.InternalError = -32603] = "InternalError", Q[Q.UrlElicitationRequired = -32042] = "UrlElicitationRequired";
})(T || (T = {}));
var F$ = P({ jsonrpc: k(P8), id: Z8.optional(), error: P({ code: s().int(), message: N(), data: K0().optional() }) }).strict();
var Gk = W0([VG, qG, L$, F$]);
var Hk = W0([L$, F$]);
var C8 = b0.strict();
var AO = a0.extend({ requestId: Z8.optional(), reason: N().optional() });
var S8 = s0.extend({ method: k("notifications/cancelled"), params: AO });
var jO = P({ src: N(), mimeType: N().optional(), sizes: n(N()).optional(), theme: y0(["light", "dark"]).optional() });
var z4 = P({ icons: n(jO).optional() });
var L9 = P({ name: N(), title: N().optional() });
var FG = L9.extend({ ...L9.shape, ...z4.shape, version: N(), websiteUrl: N().optional(), description: N().optional() });
var RO = b8(P({ applyDefaults: w0().optional() }), V0(N(), K0()));
var IO = K$((Q) => {
  if (Q && typeof Q === "object" && !Array.isArray(Q)) {
    if (Object.keys(Q).length === 0) return { form: {} };
  }
  return Q;
}, b8(P({ form: RO.optional(), url: R0.optional() }), V0(N(), K0()).optional()));
var bO = S0({ list: R0.optional(), cancel: R0.optional(), requests: S0({ sampling: S0({ createMessage: R0.optional() }).optional(), elicitation: S0({ create: R0.optional() }).optional() }).optional() });
var EO = S0({ list: R0.optional(), cancel: R0.optional(), requests: S0({ tools: S0({ call: R0.optional() }).optional() }).optional() });
var PO = P({ experimental: V0(N(), R0).optional(), sampling: P({ context: R0.optional(), tools: R0.optional() }).optional(), elicitation: IO.optional(), roots: P({ listChanged: w0().optional() }).optional(), tasks: bO.optional() });
var ZO = p0.extend({ protocolVersion: N(), capabilities: PO, clientInfo: FG });
var N$ = I0.extend({ method: k("initialize"), params: ZO });
var CO = P({ experimental: V0(N(), R0).optional(), logging: R0.optional(), completions: R0.optional(), prompts: P({ listChanged: w0().optional() }).optional(), resources: P({ subscribe: w0().optional(), listChanged: w0().optional() }).optional(), tools: P({ listChanged: w0().optional() }).optional(), tasks: EO.optional() });
var SO = b0.extend({ protocolVersion: N(), capabilities: CO, serverInfo: FG, instructions: N().optional() });
var O$ = s0.extend({ method: k("notifications/initialized"), params: a0.optional() });
var _8 = I0.extend({ method: k("ping"), params: p0.optional() });
var _O = P({ progress: s(), total: F0(s()), message: F0(N()) });
var kO = P({ ...a0.shape, ..._O.shape, progressToken: BG });
var k8 = s0.extend({ method: k("notifications/progress"), params: kO });
var vO = p0.extend({ cursor: zG.optional() });
var K4 = I0.extend({ params: vO.optional() });
var V4 = b0.extend({ nextCursor: zG.optional() });
var TO = y0(["working", "input_required", "completed", "failed", "cancelled"]);
var q4 = P({ taskId: N(), status: TO, ttl: W0([s(), B$()]), createdAt: N(), lastUpdatedAt: N(), pollInterval: F0(s()), statusMessage: F0(N()) });
var F9 = b0.extend({ task: q4 });
var xO = a0.merge(q4);
var U4 = s0.extend({ method: k("notifications/tasks/status"), params: xO });
var v8 = I0.extend({ method: k("tasks/get"), params: p0.extend({ taskId: N() }) });
var T8 = b0.merge(q4);
var x8 = I0.extend({ method: k("tasks/result"), params: p0.extend({ taskId: N() }) });
var Bk = b0.loose();
var y8 = K4.extend({ method: k("tasks/list") });
var g8 = V4.extend({ tasks: n(q4) });
var h8 = I0.extend({ method: k("tasks/cancel"), params: p0.extend({ taskId: N() }) });
var NG = b0.merge(q4);
var OG = P({ uri: N(), mimeType: F0(N()), _meta: V0(N(), K0()).optional() });
var DG = OG.extend({ text: N() });
var D$ = N().refine((Q) => {
  try {
    return atob(Q), true;
  } catch {
    return false;
  }
}, { message: "Invalid Base64 string" });
var MG = OG.extend({ blob: D$ });
var L4 = y0(["user", "assistant"]);
var N9 = P({ audience: n(L4).optional(), priority: s().min(0).max(1).optional(), lastModified: W4.datetime({ offset: true }).optional() });
var wG = P({ ...L9.shape, ...z4.shape, uri: N(), description: F0(N()), mimeType: F0(N()), annotations: N9.optional(), _meta: F0(S0({})) });
var yO = P({ ...L9.shape, ...z4.shape, uriTemplate: N(), description: F0(N()), mimeType: F0(N()), annotations: N9.optional(), _meta: F0(S0({})) });
var f8 = K4.extend({ method: k("resources/list") });
var gO = V4.extend({ resources: n(wG) });
var u8 = K4.extend({ method: k("resources/templates/list") });
var hO = V4.extend({ resourceTemplates: n(yO) });
var M$ = p0.extend({ uri: N() });
var fO = M$;
var m8 = I0.extend({ method: k("resources/read"), params: fO });
var uO = b0.extend({ contents: n(W0([DG, MG])) });
var mO = s0.extend({ method: k("notifications/resources/list_changed"), params: a0.optional() });
var lO = M$;
var cO = I0.extend({ method: k("resources/subscribe"), params: lO });
var pO = M$;
var dO = I0.extend({ method: k("resources/unsubscribe"), params: pO });
var iO = a0.extend({ uri: N() });
var nO = s0.extend({ method: k("notifications/resources/updated"), params: iO });
var oO = P({ name: N(), description: F0(N()), required: F0(w0()) });
var rO = P({ ...L9.shape, ...z4.shape, description: F0(N()), arguments: F0(n(oO)), _meta: F0(S0({})) });
var l8 = K4.extend({ method: k("prompts/list") });
var tO = V4.extend({ prompts: n(rO) });
var aO = p0.extend({ name: N(), arguments: V0(N(), N()).optional() });
var c8 = I0.extend({ method: k("prompts/get"), params: aO });
var w$ = P({ type: k("text"), text: N(), annotations: N9.optional(), _meta: V0(N(), K0()).optional() });
var A$ = P({ type: k("image"), data: D$, mimeType: N(), annotations: N9.optional(), _meta: V0(N(), K0()).optional() });
var j$ = P({ type: k("audio"), data: D$, mimeType: N(), annotations: N9.optional(), _meta: V0(N(), K0()).optional() });
var sO = P({ type: k("tool_use"), name: N(), id: N(), input: V0(N(), K0()), _meta: V0(N(), K0()).optional() });
var eO = P({ type: k("resource"), resource: W0([DG, MG]), annotations: N9.optional(), _meta: V0(N(), K0()).optional() });
var QD = wG.extend({ type: k("resource_link") });
var R$ = W0([w$, A$, j$, QD, eO]);
var XD = P({ role: L4, content: R$ });
var YD = b0.extend({ description: N().optional(), messages: n(XD) });
var $D = s0.extend({ method: k("notifications/prompts/list_changed"), params: a0.optional() });
var JD = P({ title: N().optional(), readOnlyHint: w0().optional(), destructiveHint: w0().optional(), idempotentHint: w0().optional(), openWorldHint: w0().optional() });
var WD = P({ taskSupport: y0(["required", "optional", "forbidden"]).optional() });
var AG = P({ ...L9.shape, ...z4.shape, description: N().optional(), inputSchema: P({ type: k("object"), properties: V0(N(), R0).optional(), required: n(N()).optional() }).catchall(K0()), outputSchema: P({ type: k("object"), properties: V0(N(), R0).optional(), required: n(N()).optional() }).catchall(K0()).optional(), annotations: JD.optional(), execution: WD.optional(), _meta: V0(N(), K0()).optional() });
var p8 = K4.extend({ method: k("tools/list") });
var GD = V4.extend({ tools: n(AG) });
var d8 = b0.extend({ content: n(R$).default([]), structuredContent: V0(N(), K0()).optional(), isError: w0().optional() });
var zk = d8.or(b0.extend({ toolResult: K0() }));
var HD = H4.extend({ name: N(), arguments: V0(N(), K0()).optional() });
var O9 = I0.extend({ method: k("tools/call"), params: HD });
var BD = s0.extend({ method: k("notifications/tools/list_changed"), params: a0.optional() });
var Kk = P({ autoRefresh: w0().default(true), debounceMs: s().int().nonnegative().default(300) });
var F4 = y0(["debug", "info", "notice", "warning", "error", "critical", "alert", "emergency"]);
var zD = p0.extend({ level: F4 });
var I$ = I0.extend({ method: k("logging/setLevel"), params: zD });
var KD = a0.extend({ level: F4, logger: N().optional(), data: K0() });
var VD = s0.extend({ method: k("notifications/message"), params: KD });
var qD = P({ name: N().optional() });
var UD = P({ hints: n(qD).optional(), costPriority: s().min(0).max(1).optional(), speedPriority: s().min(0).max(1).optional(), intelligencePriority: s().min(0).max(1).optional() });
var LD = P({ mode: y0(["auto", "required", "none"]).optional() });
var FD = P({ type: k("tool_result"), toolUseId: N().describe("The unique identifier for the corresponding tool call."), content: n(R$).default([]), structuredContent: P({}).loose().optional(), isError: w0().optional(), _meta: V0(N(), K0()).optional() });
var ND = z$("type", [w$, A$, j$]);
var E8 = z$("type", [w$, A$, j$, sO, FD]);
var OD = P({ role: L4, content: W0([E8, n(E8)]), _meta: V0(N(), K0()).optional() });
var DD = H4.extend({ messages: n(OD), modelPreferences: UD.optional(), systemPrompt: N().optional(), includeContext: y0(["none", "thisServer", "allServers"]).optional(), temperature: s().optional(), maxTokens: s().int(), stopSequences: n(N()).optional(), metadata: R0.optional(), tools: n(AG).optional(), toolChoice: LD.optional() });
var MD = I0.extend({ method: k("sampling/createMessage"), params: DD });
var N4 = b0.extend({ model: N(), stopReason: F0(y0(["endTurn", "stopSequence", "maxTokens"]).or(N())), role: L4, content: ND });
var b$ = b0.extend({ model: N(), stopReason: F0(y0(["endTurn", "stopSequence", "maxTokens", "toolUse"]).or(N())), role: L4, content: W0([E8, n(E8)]) });
var wD = P({ type: k("boolean"), title: N().optional(), description: N().optional(), default: w0().optional() });
var AD = P({ type: k("string"), title: N().optional(), description: N().optional(), minLength: s().optional(), maxLength: s().optional(), format: y0(["email", "uri", "date", "date-time"]).optional(), default: N().optional() });
var jD = P({ type: y0(["number", "integer"]), title: N().optional(), description: N().optional(), minimum: s().optional(), maximum: s().optional(), default: s().optional() });
var RD = P({ type: k("string"), title: N().optional(), description: N().optional(), enum: n(N()), default: N().optional() });
var ID = P({ type: k("string"), title: N().optional(), description: N().optional(), oneOf: n(P({ const: N(), title: N() })), default: N().optional() });
var bD = P({ type: k("string"), title: N().optional(), description: N().optional(), enum: n(N()), enumNames: n(N()).optional(), default: N().optional() });
var ED = W0([RD, ID]);
var PD = P({ type: k("array"), title: N().optional(), description: N().optional(), minItems: s().optional(), maxItems: s().optional(), items: P({ type: k("string"), enum: n(N()) }), default: n(N()).optional() });
var ZD = P({ type: k("array"), title: N().optional(), description: N().optional(), minItems: s().optional(), maxItems: s().optional(), items: P({ anyOf: n(P({ const: N(), title: N() })) }), default: n(N()).optional() });
var CD = W0([PD, ZD]);
var SD = W0([bD, ED, CD]);
var _D = W0([SD, wD, AD, jD]);
var kD = H4.extend({ mode: k("form").optional(), message: N(), requestedSchema: P({ type: k("object"), properties: V0(N(), _D), required: n(N()).optional() }) });
var vD = H4.extend({ mode: k("url"), message: N(), elicitationId: N(), url: N().url() });
var TD = W0([kD, vD]);
var xD = I0.extend({ method: k("elicitation/create"), params: TD });
var yD = a0.extend({ elicitationId: N() });
var gD = s0.extend({ method: k("notifications/elicitation/complete"), params: yD });
var D9 = b0.extend({ action: y0(["accept", "decline", "cancel"]), content: K$((Q) => Q === null ? void 0 : Q, V0(N(), W0([N(), s(), w0(), n(N())])).optional()) });
var hD = P({ type: k("ref/resource"), uri: N() });
var fD = P({ type: k("ref/prompt"), name: N() });
var uD = p0.extend({ ref: W0([fD, hD]), argument: P({ name: N(), value: N() }), context: P({ arguments: V0(N(), N()).optional() }).optional() });
var i8 = I0.extend({ method: k("completion/complete"), params: uD });
var mD = b0.extend({ completion: S0({ values: n(N()).max(100), total: F0(s().int()), hasMore: F0(w0()) }) });
var lD = P({ uri: N().startsWith("file://"), name: N().optional(), _meta: V0(N(), K0()).optional() });
var cD = I0.extend({ method: k("roots/list"), params: p0.optional() });
var E$ = b0.extend({ roots: n(lD) });
var pD = s0.extend({ method: k("notifications/roots/list_changed"), params: a0.optional() });
var Vk = W0([_8, N$, i8, I$, c8, l8, f8, u8, m8, cO, dO, O9, p8, v8, x8, y8, h8]);
var qk = W0([S8, k8, O$, pD, U4]);
var Uk = W0([C8, N4, b$, D9, E$, T8, g8, F9]);
var Lk = W0([_8, MD, xD, cD, v8, x8, y8, h8]);
var Fk = W0([S8, k8, VD, nO, mO, BD, $D, U4, gD]);
var Nk = W0([C8, SO, mD, YD, tO, gO, hO, uO, d8, GD, T8, g8, F9]);
var nD = new Set("ABCDEFGHIJKLMNOPQRSTUVXYZabcdefghijklmnopqrstuvxyz0123456789");
var mK = s7(Z7(), 1);
var lK = s7(uK(), 1);
var dK;
(function(Q) {
  Q.Completable = "McpCompletable";
})(dK || (dK = {}));
function aK(Q) {
  let X;
  return () => X ??= Q();
}
var OE = aK(() => i1.object({ session_id: i1.string(), ws_url: i1.string(), work_dir: i1.string().optional(), session_key: i1.string().optional() }));
function Yh({ prompt: Q, options: X }) {
  let { systemPrompt: Y, settings: $, settingSources: J, sandbox: W, ...G } = X ?? {}, H, B;
  if (Y === void 0) H = "";
  else if (typeof Y === "string") H = Y;
  else if (Y.type === "preset") B = Y.append;
  let z = G.pathToClaudeCodeExecutable;
  if (!z) {
    let T6 = wE(import.meta.url), x6 = XV(T6, "..");
    z = XV(x6, "cli.js");
  }
  process.env.CLAUDE_AGENT_SDK_VERSION = "0.2.76";
  let { abortController: K = y6(), additionalDirectories: U = [], agent: q, agents: V, allowedTools: L = [], betas: F, canUseTool: M, continue: O, cwd: A, debug: R, debugFile: Z, disallowedTools: C = [], tools: B0, env: O0, executable: d0 = g6() ? "bun" : "node", executableArgs: B6 = [], extraArgs: F1 = {}, fallbackModel: z6, enableFileCheckpointing: y1, toolConfig: K6, forkSession: h, hooks: S9, includePartialMessages: TQ, onElicitation: _9, persistSession: k9, thinking: V6, effort: l4, maxThinkingTokens: k6, maxTurns: C0, maxBudgetUsd: g1, mcpServers: v6, model: YV, outputFormat: i7, permissionMode: $V = "default", allowDangerouslySkipPermissions: JV = false, permissionPromptToolName: WV, plugins: GV, workload: n7, resume: HV, resumeSessionAt: BV, sessionId: zV, stderr: KV, strictMcpConfig: VV } = G, o7 = i7?.type === "json_schema" ? i7.schema : void 0, q6 = O0;
  if (!q6) q6 = { ...process.env };
  if (!q6.CLAUDE_CODE_ENTRYPOINT) q6.CLAUDE_CODE_ENTRYPOINT = "sdk-ts";
  if (y1) q6.CLAUDE_CODE_ENABLE_SDK_FILE_CHECKPOINTING = "true";
  if (K6?.askUserQuestion?.previewFormat) q6.CLAUDE_CODE_QUESTION_PREVIEW_FORMAT = K6.askUserQuestion.previewFormat;
  if (!z) throw Error("pathToClaudeCodeExecutable is required");
  let xQ = {}, r7 = /* @__PURE__ */ new Map();
  if (v6) for (let [T6, x6] of Object.entries(v6)) if (x6.type === "sdk" && "instance" in x6) r7.set(T6, x6.instance), xQ[T6] = { type: "sdk", name: T6 };
  else xQ[T6] = x6;
  let qV = typeof Q === "string", v9;
  if (V6) switch (V6.type) {
    case "adaptive":
      v9 = { type: "adaptive" };
      break;
    case "enabled":
      v9 = { type: "enabled", budgetTokens: V6.budgetTokens };
      break;
    case "disabled":
      v9 = { type: "disabled" };
      break;
  }
  else if (k6 !== void 0) v9 = k6 === 0 ? { type: "disabled" } : { type: "enabled", budgetTokens: k6 };
  let t7 = new y9({ abortController: K, additionalDirectories: U, agent: q, betas: F, cwd: A, debug: R, debugFile: Z, executable: d0, executableArgs: B6, extraArgs: n7 ? { ...F1, workload: n7 } : F1, pathToClaudeCodeExecutable: z, env: q6, forkSession: h, stderr: KV, thinkingConfig: v9, effort: l4, maxTurns: C0, maxBudgetUsd: g1, model: YV, fallbackModel: z6, jsonSchema: o7, permissionMode: $V, allowDangerouslySkipPermissions: JV, permissionPromptToolName: WV, continueConversation: O, resume: HV, resumeSessionAt: BV, sessionId: zV, settings: typeof $ === "object" ? G0($) : $, settingSources: J ?? [], allowedTools: L, disallowedTools: C, tools: B0, mcpServers: xQ, strictMcpConfig: VV, canUseTool: !!M, hooks: !!S9, includePartialMessages: TQ, persistSession: k9, plugins: GV, sandbox: W, spawnClaudeCodeProcess: G.spawnClaudeCodeProcess }), UV = { systemPrompt: H, appendSystemPrompt: B, agents: V, promptSuggestions: G.promptSuggestions, agentProgressSummaries: G.agentProgressSummaries }, a7 = new h9(t7, qV, M, S9, K, r7, o7, UV, _9);
  if (typeof Q === "string") t7.write(G0({ type: "user", session_id: "", message: { role: "user", content: [{ type: "text", text: Q }] }, parent_tool_use_id: null }) + `
`);
  else a7.streamInput(Q);
  return a7;
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
    out = out.replace(pattern, (m2) => {
      count += 1;
      return token(kind, m2);
    });
  }
  return { text: out, count };
}

// index.ts
import * as readline from "readline";
var ENGINE_URL = "http://localhost:8080/mcp";
var SYSTEM_PROMPT = `You are an ontology engineering assistant with MCP tools for the Open Ontologies engine.

No emoji. Plain text and markdown only.

CRITICAL: When asked to build an ontology, you will receive step-by-step instructions. Follow each step EXACTLY. Call the tools specified \u2014 do NOT just describe what you would do.

After any onto_load, always call onto_stats to verify what was loaded.
After all loads are done, always call onto_save with path "~/.open-ontologies/studio-live.ttl".`;
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
var sessionId;
function send(msg) {
  process.stdout.write(JSON.stringify(msg) + "\n");
}
async function waitForEngine(maxRetries = 15) {
  for (let i2 = 0; i2 < maxRetries; i2++) {
    try {
      const resp = await fetch(ENGINE_URL, {
        method: "POST",
        headers: { "Content-Type": "application/json", "Accept": "application/json, text/event-stream" },
        body: JSON.stringify({
          jsonrpc: "2.0",
          id: 1,
          method: "initialize",
          params: { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "probe", version: "1.0.0" } }
        })
      });
      if (resp.ok) return true;
    } catch {
    }
    await new Promise((r) => setTimeout(r, 1e3));
  }
  return false;
}
async function runTurn(prompt) {
  let mutated = false;
  const q = Yh({
    prompt,
    options: {
      systemPrompt: SYSTEM_PROMPT,
      model: "claude-opus-4-8",
      mcpServers: { "ontology-engine": { type: "http", url: ENGINE_URL } },
      allowedTools: ["mcp__ontology-engine__*"],
      tools: [],
      persistSession: true,
      ...sessionId ? { resume: sessionId } : {},
      permissionMode: "bypassPermissions",
      allowDangerouslySkipPermissions: true,
      maxTurns: 15
    }
  });
  for await (const message of q) {
    if ("session_id" in message && message.session_id) {
      if (!sessionId) {
        sessionId = message.session_id;
        send({ type: "session", sessionId: message.session_id });
      }
    }
    switch (message.type) {
      case "assistant": {
        const content = message.message?.content;
        if (Array.isArray(content)) {
          for (const block of content) {
            if (block.type === "text" && block.text) {
              send({ type: "text", content: block.text });
            }
            if (block.type === "tool_use") {
              send({ type: "tool_call", tool: block.name, input: block.input });
              if ([...MUTATION_TOOLS].some((t) => block.name === t || block.name.endsWith(`__${t}`))) {
                mutated = true;
              }
            }
          }
        }
        break;
      }
      case "result": {
        if (message.subtype !== "success") {
          const errors = "errors" in message ? message.errors : [];
          send({ type: "error", error: errors && errors.length > 0 ? errors.join("; ") : `Agent error: ${message.subtype}` });
        }
        break;
      }
      case "system":
        break;
    }
  }
  return mutated;
}
function isBuildRequest(msg) {
  const lower = msg.toLowerCase();
  return (lower.includes("build") || lower.includes("create") || lower.includes("make") || lower.includes("generate")) && (lower.includes("ontology") || lower.includes("about"));
}
function isSketchRequest(msg) {
  return msg.toLowerCase().includes("sketch");
}
function extractDomain(msg) {
  const patterns = [
    /(?:about|for|on|of)\s+(.+)/i,
    /(?:build|create|make|generate|sketch)\s+(?:a\s+|an\s+|the\s+)?(?:\w+\s+)?(?:ontology\s+)?(?:about|for|on|of)\s+(.+)/i
  ];
  for (const p2 of patterns) {
    const m2 = msg.match(p2);
    if (m2) {
      const match = m2[2] || m2[1];
      if (match) return match.trim().replace(/[.!?]+$/, "");
    }
  }
  return msg.replace(/^(build|create|make|generate|sketch)\s+(an?\s+)?ontology\s*/i, "").trim() || msg;
}
async function handleBuild(domain) {
  const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const prefix = `@prefix : <http://example.org/${ns}#> .`;
  const DEEPEN = (branchDesc) => `Call onto_query with this SPARQL to find leaf classes in ${branchDesc}:
SELECT ?leaf ?label WHERE { ?leaf rdfs:subClassOf+ ?branch . ?branch rdfs:subClassOf :${branchDesc.includes("FIRST") ? "" : ""} . FILTER NOT EXISTS { ?child rdfs:subClassOf ?leaf } . OPTIONAL { ?leaf rdfs:label ?label } } LIMIT 30

Then call onto_load with Turtle using the SAME namespace ${prefix} adding DEEPER subclass chains. For each leaf class:
1. Add 3-5 rdfs:subClassOf children
2. For each of THOSE children, add 2-4 more subclasses (grandchildren of the leaf)
3. If possible, add one more level below that

The goal is DEPTH not width. Each new class needs rdfs:label and rdfs:comment.
IMPORTANT: Add at most 80-120 classes in this step. Call onto_stats after. Do NOT save yet.`;
  const steps = [
    {
      label: "Step 1: Foundation \u2014 root + 5 levels deep",
      prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with Turtle containing:
- An owl:Ontology declaration
- A root class :${ns.charAt(0).toUpperCase() + ns.slice(1).replace(/-./g, (m2) => m2[1].toUpperCase())}
- 6-10 major branch classes as rdfs:subClassOf the root (Level 1)
- For each branch, 3-5 subclasses (Level 2)
- For each of those, 2-4 further subclasses (Level 3)
- For at least half of Level 3, add 2-3 more subclasses (Level 4)
- For at least a quarter of Level 4, add 2 more subclasses (Level 5)

Structure this as a DEEP tree, not a wide one. Every class MUST have rdfs:label and rdfs:comment.

Call onto_stats after. Do NOT save yet \u2014 many more steps coming.`
    },
    {
      label: "Step 2: Deepen \u2014 first major branch to maximum depth",
      prompt: DEEPEN("the FIRST major branch (the first child of the root)")
    },
    {
      label: "Step 3: Deepen \u2014 second major branch to maximum depth",
      prompt: DEEPEN("the SECOND major branch")
    },
    {
      label: "Step 4: Deepen \u2014 third major branch to maximum depth",
      prompt: DEEPEN("the THIRD major branch")
    },
    {
      label: "Step 5: Deepen \u2014 fourth and fifth major branches",
      prompt: DEEPEN("the FOURTH and FIFTH major branches")
    },
    {
      label: "Step 6: Deepen \u2014 all remaining branches",
      prompt: DEEPEN("ALL remaining major branches that have not been deepened yet")
    },
    {
      label: "Step 7: Deepen \u2014 second pass on shallow areas",
      prompt: `Run this SPARQL to measure depth per root branch:
SELECT ?branch (MAX(?depth) AS ?maxDepth) WHERE { ?class rdfs:subClassOf+ ?branch . ?branch rdfs:subClassOf ?root . ?root a owl:Class . FILTER NOT EXISTS { ?root rdfs:subClassOf ?x . ?x a owl:Class } . { SELECT ?class (COUNT(?mid) AS ?depth) WHERE { ?class rdfs:subClassOf+ ?mid } GROUP BY ?class } } GROUP BY ?branch ORDER BY ?maxDepth LIMIT 20

For any branch with maxDepth < 7, find its leaf classes and add 2-3 more levels of subclasses below them (chain: A subClassOf B subClassOf C).
Also: are there any major subtypes or categories missing entirely? Add them now as deep chains, not flat siblings.
Call onto_load with the Turtle. Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 8: Object properties \u2014 structural relationships",
      prompt: `Now add object properties. Call onto_load with Turtle containing 50-70 owl:ObjectProperty declarations.

EVERY property MUST have: rdfs:domain, rdfs:range, rdfs:label, rdfs:comment.

Cover ALL relationship types:
- Compositional: hasPart/isPartOf, contains/isContainedIn, hasComponent/isComponentOf
- Causal: causes, prevents, triggers, treats, inhibits, enables
- Associative: isAssociatedWith, isRelatedTo, dependsOn, influences
- Hierarchical: hasSubtype, isExampleOf, instantiates
- Build rdfs:subPropertyOf hierarchies (3-4 levels deep)
- Add owl:inverseOf for EVERY directional property
- Mark owl:TransitiveProperty, owl:SymmetricProperty, owl:FunctionalProperty

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 9: Object properties \u2014 roles, temporal, spatial",
      prompt: `Add MORE object properties. Call onto_load with Turtle containing 50-70 MORE owl:ObjectProperty declarations.

Focus on what's MISSING \u2014 look at the classes and ask "how does X relate to Y?" for every pair of branches:
- Role/participation: hasRole, participatesIn, performs, undergoes, produces, consumes
- Temporal: precedes, follows, during, overlaps, startsWith, endsWith
- Spatial: isLocatedIn, isNear, surrounds, isAdjacentTo, isWithin
- Ownership: owns, belongsTo, isOwnedBy, manages, controls
- Agent: hasAgent, hasPatient, hasBeneficiary, hasInstrument
- More rdfs:subPropertyOf hierarchies and owl:inverseOf pairs

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 10: Datatype properties \u2014 all attributes",
      prompt: `Add datatype properties. Call onto_load with Turtle containing 40-60 owl:DatatypeProperty declarations.

Each with: rdfs:domain, rdfs:range (xsd types), rdfs:label, rdfs:comment.

Go through EVERY major branch and add ALL attributes:
- Identifiers, names, codes, labels, descriptions, titles
- Dates (birth, creation, modification, expiry, start, end)
- Quantities (weight, height, length, count, duration, price, score, rating, percentage)
- Measurements (temperature, speed, volume, area, concentration)
- Boolean flags (isActive, isVerified, isPublic, isRequired, isOptional, isDeprecated)
- Statuses, categories, priorities, levels, grades
- Text fields (notes, comments, instructions, definitions)

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 11: Axioms \u2014 disjoints everywhere",
      prompt: `Add disjoint axioms. Call onto_load with Turtle containing owl:disjointWith between ALL sibling classes that cannot overlap.

Go through EVERY branch systematically:
- Root children: all major branches that are mutually exclusive
- Within each branch: siblings that cannot overlap
- Target: 60+ disjoint axiom pairs minimum

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 12: Individuals \u2014 real-world examples",
      prompt: `Add named individuals. Call onto_load with Turtle containing 25-40 owl:NamedIndividual instances.

Spread them across ALL major branches \u2014 at least 3-4 individuals per branch.
Each individual needs:
- rdf:type (the most specific class)
- rdfs:label and rdfs:comment
- 3-5 property values (both object and datatype properties)

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 13: Reason + save",
      prompt: `Final step. Run:
1. onto_reason with profile "rdfs"
2. onto_stats \u2014 report the final counts
3. onto_save with path "~/.open-ontologies/studio-live.ttl"

Report the final ontology statistics.`
    }
  ];
  send({ type: "text", content: `**Building maximum-depth ontology: ${domain}** (${steps.length} steps)
` });
  send({ type: "progress", step: 0, total: steps.length, label: "Starting build..." });
  for (let i2 = 0; i2 < steps.length; i2++) {
    const step = steps[i2];
    send({ type: "progress", step: i2 + 1, total: steps.length, label: step.label });
    send({ type: "text", content: `
---
**${step.label}**` });
    try {
      await runTurn(step.prompt);
    } catch (e2) {
      send({ type: "text", content: `Step failed: ${e2}. Continuing...` });
    }
  }
  send({ type: "progress", step: steps.length, total: steps.length, label: "Build complete" });
  send({ type: "text", content: `
---
**Build complete.** The graph should now be visible in the tree view.` });
}
async function handleSketch(domain) {
  const ns = domain.toLowerCase().replace(/[^a-z0-9]+/g, "-");
  const prefix = `@prefix : <http://example.org/${ns}#> .`;
  const steps = [
    {
      label: "Step 1/5: Foundation \u2014 root + 4 levels deep",
      prompt: `Build an ontology about "${domain}". Use namespace ${prefix}

Call onto_clear. Then call onto_load with ONE Turtle block containing:
- An owl:Ontology declaration
- A root class for the domain
- 5-8 major branch classes under the root (Level 1)
- For each branch, 3-4 subclasses (Level 2)
- For each of those, 2-3 further subclasses (Level 3)
- For at least half of Level 3, add 2 more subclasses (Level 4)

Structure as a DEEP tree \u2014 prioritize depth over width. Every class MUST have rdfs:label and rdfs:comment.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 2/5: Deepen + properties",
      prompt: `Call onto_query to find leaf classes:
SELECT ?leaf ?label WHERE { ?leaf a owl:Class . FILTER NOT EXISTS { ?child rdfs:subClassOf ?leaf } . OPTIONAL { ?leaf rdfs:label ?label } } LIMIT 30

Then call onto_load with Turtle using namespace ${prefix} adding:
- For each leaf that can be subdivided: add 2-3 subclasses, and for each of those add 1-2 more subclasses (create chains 2-3 levels deeper, not just one flat level)
- 15-25 owl:ObjectProperty each with rdfs:domain, rdfs:range, rdfs:label, rdfs:comment
- owl:inverseOf pairs for directional properties
- 8-12 owl:DatatypeProperty with rdfs:domain, rdfs:range (xsd types), rdfs:label

The goal is DEPTH \u2014 chain subclasses 2-3 levels deep from the current leaves.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 3/5: Axioms + individuals",
      prompt: `Call onto_load with Turtle using namespace ${prefix} adding:
- owl:disjointWith between sibling classes that cannot overlap (15+ pairs)
- 12-20 owl:NamedIndividual spread across different branches, each with:
  - rdf:type (the most specific class)
  - rdfs:label and rdfs:comment
  - 2-4 property values (both object and datatype properties)

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 4/5: Verify + fix gaps",
      prompt: `Run this SPARQL to measure max depth:
SELECT (MAX(?depth) AS ?maxDepth) WHERE { { SELECT ?class (COUNT(?mid) AS ?depth) WHERE { ?class rdfs:subClassOf+ ?mid . ?mid rdfs:subClassOf+ ?root . ?root a owl:Class . FILTER NOT EXISTS { ?root rdfs:subClassOf ?x . ?x a owl:Class } } GROUP BY ?class } }

Also call onto_stats to check individual count.

If max depth < 5: pick the 3 shallowest leaf classes and call onto_load with Turtle adding 2-3 levels of subclasses below each (chain them: A subClassOf B subClassOf C).
If individuals < 10: call onto_load adding more individuals.

Call onto_stats after. Do NOT save yet.`
    },
    {
      label: "Step 5/5: Reason + save",
      prompt: `Run onto_reason (profile "rdfs"), then onto_stats, then onto_save ("~/.open-ontologies/studio-live.ttl"). Report final statistics including class count, property count, individual count, max depth, and triple count.`
    }
  ];
  send({ type: "text", content: `**Sketching ontology: ${domain}** (${steps.length} steps)
` });
  send({ type: "progress", step: 0, total: steps.length, label: "Starting sketch..." });
  for (let i2 = 0; i2 < steps.length; i2++) {
    const step = steps[i2];
    send({ type: "progress", step: i2 + 1, total: steps.length, label: step.label });
    send({ type: "text", content: `
---
**${step.label}**` });
    try {
      await runTurn(step.prompt);
    } catch (e2) {
      send({ type: "text", content: `Step failed: ${e2}. Continuing...` });
    }
  }
  send({ type: "progress", step: steps.length, total: steps.length, label: "Sketch complete" });
  send({ type: "text", content: `
---
**Sketch complete.** Use /expand to deepen any branch.` });
}
async function handleMessage(userMessage, mode = "sketch") {
  const tok = tokeniseInbound(userMessage);
  if (tok.count > 0) {
    userMessage = tok.text;
    send({
      type: "text",
      content: `_${tok.count} sensitive value(s) tokenised before the model saw your question._
`
    });
  }
  try {
    const isBuildLike = isBuildRequest(userMessage) || isSketchRequest(userMessage);
    if (isBuildLike) {
      const domain = extractDomain(userMessage);
      sessionId = void 0;
      if (mode === "sketch") {
        await handleSketch(domain);
      } else {
        await handleBuild(domain);
      }
      send({ type: "done", mutated: true });
    } else {
      const mutated = await runTurn(userMessage);
      send({ type: "done", mutated });
    }
  } catch (e2) {
    send({ type: "error", error: String(e2) });
    send({ type: "done", mutated: false });
  }
}
async function main() {
  const engineReady = await waitForEngine();
  if (!engineReady) {
    send({ type: "error", error: "Engine not reachable after 15 retries" });
  }
  send({ type: "ready" });
  const rl = readline.createInterface({ input: process.stdin });
  rl.on("line", async (line) => {
    try {
      const msg = JSON.parse(line);
      if (msg.type === "chat") {
        await handleMessage(msg.message, msg.mode || "sketch");
      } else if (msg.type === "reset") {
        sessionId = void 0;
        send({ type: "reset_done" });
      }
    } catch (e2) {
      send({ type: "error", error: String(e2) });
    }
  });
}
main().catch((e2) => {
  send({ type: "error", error: String(e2) });
  process.exit(1);
});
