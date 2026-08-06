//#region node_modules/fake-indexeddb/build/esm/lib/errors.js
var e = {
	AbortError: "A request was aborted, for example through a call to IDBTransaction.abort.",
	ConstraintError: "A mutation operation in the transaction failed because a constraint was not satisfied. For example, an object such as an object store or index already exists and a request attempted to create a new one.",
	DataCloneError: "The data being stored could not be cloned by the internal structured cloning algorithm.",
	DataError: "Data provided to an operation does not meet requirements.",
	InvalidAccessError: "An invalid operation was performed on an object. For example transaction creation attempt was made, but an empty scope was provided.",
	InvalidStateError: "An operation was called on an object on which it is not allowed or at a time when it is not allowed. Also occurs if a request is made on a source object that has been deleted or removed. Use TransactionInactiveError or ReadOnlyError when possible, as they are more specific variations of InvalidStateError.",
	NotFoundError: "The operation failed because the requested database object could not be found. For example, an object store did not exist but was being opened.",
	ReadOnlyError: "The mutating operation was attempted in a \"readonly\" transaction.",
	TransactionInactiveError: "A request was placed against a transaction which is currently not active, or which is finished.",
	SyntaxError: "The keypath argument contains an invalid key path",
	VersionError: "An attempt was made to open a database using a lower version than the existing version."
}, t = (e, t) => {
	Object.defineProperty(e, "code", {
		value: t,
		writable: !1,
		enumerable: !0,
		configurable: !1
	});
}, n = class extends DOMException {
	constructor(t = e.AbortError) {
		super(t, "AbortError");
	}
}, r = class extends DOMException {
	constructor(t = e.ConstraintError) {
		super(t, "ConstraintError");
	}
}, i = class extends DOMException {
	constructor(n = e.DataError) {
		super(n, "DataError"), t(this, 0);
	}
}, a = class extends DOMException {
	constructor(t = e.InvalidAccessError) {
		super(t, "InvalidAccessError");
	}
}, o = class extends DOMException {
	constructor(n = e.InvalidStateError) {
		super(n, "InvalidStateError"), t(this, 11);
	}
}, s = class extends DOMException {
	constructor(t = e.NotFoundError) {
		super(t, "NotFoundError");
	}
}, c = class extends DOMException {
	constructor(t = e.ReadOnlyError) {
		super(t, "ReadOnlyError");
	}
}, l = class extends DOMException {
	constructor(n = e.VersionError) {
		super(n, "SyntaxError"), t(this, 12);
	}
}, u = class extends DOMException {
	constructor(n = e.TransactionInactiveError) {
		super(n, "TransactionInactiveError"), t(this, 0);
	}
}, d = class extends DOMException {
	constructor(t = e.VersionError) {
		super(t, "VersionError");
	}
};
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/isSharedArrayBuffer.js
function ee(e) {
	return typeof SharedArrayBuffer < "u" && e instanceof SharedArrayBuffer;
}
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/valueToKeyWithoutThrowing.js
var f = Symbol("INVALID_TYPE"), p = Symbol("INVALID_VALUE"), m = (e, t) => {
	if (typeof e == "number") return isNaN(e) ? p : e;
	if (Object.prototype.toString.call(e) === "[object Date]") {
		let t = e.valueOf();
		return isNaN(t) ? p : new Date(t);
	} else if (typeof e == "string") return e;
	else if (e instanceof ArrayBuffer || ee(e) || typeof ArrayBuffer < "u" && ArrayBuffer.isView && ArrayBuffer.isView(e)) {
		if ("detached" in e ? e.detached : e.byteLength === 0) return p;
		let t, n = 0, r = 0;
		return e instanceof ArrayBuffer || ee(e) ? (t = e, r = e.byteLength) : (t = e.buffer, n = e.byteOffset, r = e.byteLength), t.slice(n, n + r);
	} else if (Array.isArray(e)) {
		if (t === void 0) t = /* @__PURE__ */ new Set();
		else if (t.has(e)) return p;
		t.add(e);
		let n = !1, r = Array.from({ length: e.length }, (r, i) => {
			if (n) return;
			if (!Object.hasOwn(e, i)) {
				n = !0;
				return;
			}
			let a = e[i], o = m(a, t);
			if (o === p || o === f) {
				n = !0;
				return;
			}
			return o;
		});
		return n ? p : r;
	} else return f;
}, h = (e, t) => {
	let n = m(e, t);
	if (n === p || n === f) throw new i();
	return n;
}, te = (e) => {
	if (typeof e == "number") return "Number";
	if (Object.prototype.toString.call(e) === "[object Date]") return "Date";
	if (Array.isArray(e)) return "Array";
	if (typeof e == "string") return "String";
	if (e instanceof ArrayBuffer) return "Binary";
	throw new i();
}, g = (e, t) => {
	if (t === void 0) throw TypeError();
	e = h(e), t = h(t);
	let n = te(e), r = te(t);
	if (n !== r) return n === "Array" || n === "Binary" && (r === "String" || r === "Date" || r === "Number") || n === "String" && (r === "Date" || r === "Number") || n === "Date" && r === "Number" ? 1 : -1;
	if (n === "Binary" && (e = new Uint8Array(e), t = new Uint8Array(t)), n === "Array" || n === "Binary") {
		let n = Math.min(e.length, t.length);
		for (let r = 0; r < n; r++) {
			let n = g(e[r], t[r]);
			if (n !== 0) return n;
		}
		return e.length > t.length ? 1 : e.length < t.length ? -1 : 0;
	}
	if (n === "Date") {
		if (e.getTime() === t.getTime()) return 0;
	} else if (e === t) return 0;
	return e > t ? 1 : -1;
}, _ = class e {
	static only(t) {
		if (arguments.length === 0) throw TypeError();
		return t = h(t), new e(t, t, !1, !1);
	}
	static lowerBound(t, n = !1) {
		if (arguments.length === 0) throw TypeError();
		return t = h(t), new e(t, void 0, n, !0);
	}
	static upperBound(t, n = !1) {
		if (arguments.length === 0) throw TypeError();
		return t = h(t), new e(void 0, t, !0, n);
	}
	static bound(t, n, r = !1, a = !1) {
		if (arguments.length < 2) throw TypeError();
		let o = g(t, n);
		if (o === 1 || o === 0 && (r || a)) throw new i();
		return t = h(t), n = h(n), new e(t, n, r, a);
	}
	constructor(e, t, n, r) {
		this.lower = e, this.upper = t, this.lowerOpen = n, this.upperOpen = r;
	}
	includes(e) {
		if (arguments.length === 0) throw TypeError();
		if (e = h(e), this.lower !== void 0) {
			let t = g(this.lower, e);
			if (t === 1 || t === 0 && this.lowerOpen) return !1;
		}
		if (this.upper !== void 0) {
			let t = g(this.upper, e);
			if (t === -1 || t === 0 && this.upperOpen) return !1;
		}
		return !0;
	}
	get [Symbol.toStringTag]() {
		return "IDBKeyRange";
	}
}, v = (e, t) => {
	if (Array.isArray(e)) {
		let n = [];
		for (let r of e) {
			r != null && typeof r != "string" && r.toString && (r = r.toString());
			let e = v(r, t).key;
			n.push(h(e));
		}
		return {
			type: "found",
			key: n
		};
	}
	if (e === "") return {
		type: "found",
		key: t
	};
	let n = e, r = t;
	for (; n !== null;) {
		let e, t = n.indexOf(".");
		if (t >= 0 ? (e = n.slice(0, t), n = n.slice(t + 1)) : (e = n, n = null), !(e === "length" && (typeof r == "string" || Array.isArray(r)) || (e === "size" || e === "type") && typeof Blob < "u" && r instanceof Blob || (e === "name" || e === "lastModified") && typeof File < "u" && r instanceof File) && (typeof r != "object" || !r || !Object.hasOwn(r, e))) return { type: "notFound" };
		r = r[e];
	}
	return {
		type: "found",
		key: r
	};
};
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/cloneValueForInsertion.js
function ne(e, t) {
	if (t._state !== "active") throw Error("Assert: transaction state is active");
	t._state = "inactive";
	try {
		return structuredClone(e);
	} finally {
		t._state = "active";
	}
}
//#endregion
//#region node_modules/fake-indexeddb/build/esm/FDBCursor.js
var y = (e) => e.source instanceof F ? e.source : e.source.objectStore, b = (e, t, n) => {
	let r = e === void 0 ? void 0 : e.lower, i = e === void 0 ? void 0 : e.upper;
	for (let e of t) e !== void 0 && (r === void 0 || g(r, e) === 1) && (r = e);
	for (let e of n) e !== void 0 && (i === void 0 || g(i, e) === -1) && (i = e);
	if (r !== void 0 && i !== void 0) return _.bound(r, i);
	if (r !== void 0) return _.lowerBound(r);
	if (i !== void 0) return _.upperBound(i);
}, x = class {
	_gotValue = !1;
	_position = void 0;
	_objectStorePosition = void 0;
	_keyOnly = !1;
	_key = void 0;
	_primaryKey = void 0;
	constructor(e, t, n = "next", r, i = !1) {
		this._range = t, this._source = e, this._direction = n, this._request = r, this._keyOnly = i;
	}
	get source() {
		return this._source;
	}
	set source(e) {}
	get request() {
		return this._request;
	}
	set request(e) {}
	get direction() {
		return this._direction;
	}
	set direction(e) {}
	get key() {
		return this._key;
	}
	set key(e) {}
	get primaryKey() {
		return this._primaryKey;
	}
	set primaryKey(e) {}
	_iterate(e, t) {
		let n = this.source instanceof F, r = this.source instanceof F ? this.source._rawObjectStore.records : this.source._rawIndex.records, i;
		if (this.direction === "next") {
			let a = b(this._range, [e, this._position], []);
			for (let o of r.values(a)) {
				let r = e === void 0 ? void 0 : g(o.key, e), a = this._position === void 0 ? void 0 : g(o.key, this._position);
				if (!(e !== void 0 && r === -1)) {
					if (t !== void 0) {
						if (r === -1) continue;
						let e = g(o.value, t);
						if (r === 0 && e === -1) continue;
					}
					if (!(this._position !== void 0 && n && a !== 1) && !(this._position !== void 0 && !n && (a === -1 || a === 0 && g(o.value, this._objectStorePosition) !== 1)) && !(this._range !== void 0 && !this._range.includes(o.key))) {
						i = o;
						break;
					}
				}
			}
		} else if (this.direction === "nextunique") {
			let t = b(this._range, [e, this._position], []);
			for (let n of r.values(t)) if (!(e !== void 0 && g(n.key, e) === -1) && !(this._position !== void 0 && g(n.key, this._position) !== 1) && !(this._range !== void 0 && !this._range.includes(n.key))) {
				i = n;
				break;
			}
		} else if (this.direction === "prev") {
			let a = b(this._range, [], [e, this._position]);
			for (let o of r.values(a, "prev")) {
				let r = e === void 0 ? void 0 : g(o.key, e), a = this._position === void 0 ? void 0 : g(o.key, this._position);
				if (!(e !== void 0 && r === 1)) {
					if (t !== void 0) {
						if (r === 1) continue;
						let e = g(o.value, t);
						if (r === 0 && e === 1) continue;
					}
					if (!(this._position !== void 0 && n && a !== -1) && !(this._position !== void 0 && !n && (a === 1 || a === 0 && g(o.value, this._objectStorePosition) !== -1)) && !(this._range !== void 0 && !this._range.includes(o.key))) {
						i = o;
						break;
					}
				}
			}
		} else if (this.direction === "prevunique") {
			let t, n = b(this._range, [], [e, this._position]);
			for (let i of r.values(n, "prev")) if (!(e !== void 0 && g(i.key, e) === 1) && !(this._position !== void 0 && g(i.key, this._position) !== -1) && !(this._range !== void 0 && !this._range.includes(i.key))) {
				t = i;
				break;
			}
			t && (i = r.get(t.key));
		}
		let a;
		if (!i) this._key = void 0, n || (this._objectStorePosition = void 0), !this._keyOnly && this.toString() === "[object IDBCursorWithValue]" && (this.value = void 0), a = null;
		else {
			if (this._position = i.key, n || (this._objectStorePosition = i.value), this._key = i.key, n) this._primaryKey = structuredClone(i.key), !this._keyOnly && this.toString() === "[object IDBCursorWithValue]" && (this.value = structuredClone(i.value));
			else if (this._primaryKey = structuredClone(i.value), !this._keyOnly && this.toString() === "[object IDBCursorWithValue]") {
				if (this.source instanceof F) throw Error("This should never happen");
				let e = this.source.objectStore._rawObjectStore.getValue(i.value);
				this.value = structuredClone(e);
			}
			this._gotValue = !0, a = this;
		}
		return a;
	}
	update(e) {
		if (e === void 0) throw TypeError();
		let t = y(this), n = Object.hasOwn(this.source, "_rawIndex") ? this.primaryKey : this._position, r = t.transaction;
		if (r._state !== "active") throw new u();
		if (r.mode === "readonly") throw new c();
		if (t._rawObjectStore.deleted || !(this.source instanceof F) && this.source._rawIndex.deleted || !this._gotValue || !Object.hasOwn(this, "value")) throw new o();
		let a = ne(e, r);
		if (t.keyPath !== null) {
			let e;
			try {
				e = v(t.keyPath, a).key;
			} catch {}
			if (g(e, n) !== 0) throw new i();
		}
		let s = {
			key: n,
			value: a
		};
		return r._execRequestAsync({
			operation: t._rawObjectStore.storeRecord.bind(t._rawObjectStore, s, !1, r._rollbackLog),
			source: this
		});
	}
	advance(e) {
		if (!Number.isInteger(e) || e <= 0) throw TypeError();
		let t = y(this), n = t.transaction;
		if (n._state !== "active") throw new u();
		if (t._rawObjectStore.deleted || !(this.source instanceof F) && this.source._rawIndex.deleted || !this._gotValue) throw new o();
		this._request && (this._request.readyState = "pending"), n._execRequestAsync({
			operation: () => {
				let t;
				for (let n = 0; n < e && (t = this._iterate(), t); n++);
				return t;
			},
			request: this._request,
			source: this.source
		}), this._gotValue = !1;
	}
	continue(e) {
		let t = y(this), n = t.transaction;
		if (n._state !== "active") throw new u();
		if (t._rawObjectStore.deleted || !(this.source instanceof F) && this.source._rawIndex.deleted || !this._gotValue) throw new o();
		if (e !== void 0) {
			e = h(e);
			let t = g(e, this._position);
			if (t <= 0 && (this.direction === "next" || this.direction === "nextunique") || t >= 0 && (this.direction === "prev" || this.direction === "prevunique")) throw new i();
		}
		this._request && (this._request.readyState = "pending"), n._execRequestAsync({
			operation: this._iterate.bind(this, e),
			request: this._request,
			source: this.source
		}), this._gotValue = !1;
	}
	continuePrimaryKey(e, t) {
		let n = y(this), r = n.transaction;
		if (r._state !== "active") throw new u();
		if (n._rawObjectStore.deleted || !(this.source instanceof F) && this.source._rawIndex.deleted) throw new o();
		if (this.source instanceof F || this.direction !== "next" && this.direction !== "prev") throw new a();
		if (!this._gotValue) throw new o();
		if (e === void 0 || t === void 0) throw new i();
		e = h(e);
		let s = g(e, this._position);
		if (s === -1 && this.direction === "next" || s === 1 && this.direction === "prev") throw new i();
		let c = g(t, this._objectStorePosition);
		if (s === 0 && (c <= 0 && this.direction === "next" || c >= 0 && this.direction === "prev")) throw new i();
		this._request && (this._request.readyState = "pending"), r._execRequestAsync({
			operation: this._iterate.bind(this, e, t),
			request: this._request,
			source: this.source
		}), this._gotValue = !1;
	}
	delete() {
		let e = y(this), t = Object.hasOwn(this.source, "_rawIndex") ? this.primaryKey : this._position, n = e.transaction;
		if (n._state !== "active") throw new u();
		if (n.mode === "readonly") throw new c();
		if (e._rawObjectStore.deleted || !(this.source instanceof F) && this.source._rawIndex.deleted || !this._gotValue || !Object.hasOwn(this, "value")) throw new o();
		return n._execRequestAsync({
			operation: e._rawObjectStore.deleteRecord.bind(e._rawObjectStore, t, n._rollbackLog),
			source: this
		});
	}
	get [Symbol.toStringTag]() {
		return "IDBCursor";
	}
}, S = class extends x {
	value = void 0;
	constructor(e, t, n, r) {
		super(e, t, n, r);
	}
	get [Symbol.toStringTag]() {
		return "IDBCursorWithValue";
	}
}, re = (e, t) => e.immediatePropagationStopped || e.eventPhase === e.CAPTURING_PHASE && t.capture === !1 || e.eventPhase === e.BUBBLING_PHASE && t.capture === !0, C = (e, t) => {
	e.currentTarget = t;
	let n = [], r = (t) => {
		try {
			t.call(e.currentTarget, e);
		} catch (e) {
			n.push(e);
		}
	};
	for (let n of t.listeners.slice()) e.type !== n.type || re(e, n) || r(n.callback);
	let i = {
		abort: "onabort",
		blocked: "onblocked",
		close: "onclose",
		complete: "oncomplete",
		error: "onerror",
		success: "onsuccess",
		upgradeneeded: "onupgradeneeded",
		versionchange: "onversionchange"
	}[e.type];
	if (i === void 0) throw Error(`Unknown event type: "${e.type}"`);
	let a = e.currentTarget[i];
	if (a) {
		let t = {
			callback: a,
			capture: !1,
			type: e.type
		};
		re(e, t) || r(t.callback);
	}
	if (n.length) throw AggregateError(n);
}, w = class {
	listeners = [];
	addEventListener(e, t, n = !1) {
		this.listeners.push({
			callback: t,
			capture: n,
			type: e
		});
	}
	removeEventListener(e, t, n = !1) {
		let r = this.listeners.findIndex((r) => r.type === e && r.callback === t && r.capture === n);
		this.listeners.splice(r, 1);
	}
	dispatchEvent(e) {
		if (e.dispatched || !e.initialized) throw new o("The object is in an invalid state.");
		e.isTrusted = !1, e.dispatched = !0, e.target = this, e.eventPhase = e.CAPTURING_PHASE;
		for (let t of e.eventPath) e.propagationStopped || C(e, t);
		if (e.eventPhase = e.AT_TARGET, e.propagationStopped || C(e, e.target), e.bubbles) {
			e.eventPath.reverse(), e.eventPhase = e.BUBBLING_PHASE;
			for (let t of e.eventPath) e.propagationStopped || C(e, t);
		}
		return e.dispatched = !1, e.eventPhase = e.NONE, e.currentTarget = null, !e.canceled;
	}
}, T = class extends w {
	_result = null;
	_error = null;
	source = null;
	transaction = null;
	readyState = "pending";
	onsuccess = null;
	onerror = null;
	get error() {
		if (this.readyState === "pending") throw new o();
		return this._error;
	}
	set error(e) {
		this._error = e;
	}
	get result() {
		if (this.readyState === "pending") throw new o();
		return this._result;
	}
	set result(e) {
		this._result = e;
	}
	get [Symbol.toStringTag]() {
		return "IDBRequest";
	}
}, E = class {
	constructor(...e) {
		this._values = e;
		for (let t = 0; t < e.length; t++) this[t] = e[t];
	}
	contains(e) {
		return this._values.includes(e);
	}
	item(e) {
		return e < 0 || e >= this._values.length ? null : this._values[e];
	}
	get length() {
		return this._values.length;
	}
	[Symbol.iterator]() {
		return this._values[Symbol.iterator]();
	}
	_push(...e) {
		for (let t = 0; t < e.length; t++) this[this._values.length + t] = e[t];
		this._values.push(...e);
	}
	_sort(...e) {
		this._values.sort(...e);
		for (let e = 0; e < this._values.length; e++) this[e] = this._values[e];
		return this;
	}
}, D = (e, t = !1) => {
	if (e instanceof _) return e;
	if (e == null) {
		if (t) throw new i();
		return new _(void 0, void 0, !1, !1);
	}
	let n = h(e);
	return _.only(n);
}, ie = (e) => typeof e == "object" && e ? e + "" : e;
function ae(e) {
	return Array.isArray(e) ? e.map(ie) : ie(e);
}
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/isPotentiallyValidKeyRange.js
var oe = (e) => e instanceof _ || m(e) !== f, O = (e, t) => {
	if (isNaN(e) || e < 0 || e > (t === "unsigned long" ? 4294967295 : 9007199254740991)) throw TypeError();
	if (e >= 0) return Math.floor(e);
}, k = (e, t, n) => {
	let r, i;
	if (e == null || oe(e)) r = e, n > 1 && t !== void 0 && (t = O(t, "unsigned long"));
	else {
		let n = e;
		n.query !== void 0 && (r = n.query), n.count !== void 0 && (t = O(n.count, "unsigned long")), n.direction !== void 0 && (i = n.direction);
	}
	return {
		query: r,
		count: t,
		direction: i
	};
}, A = (e) => {
	if (e._rawIndex.deleted || e.objectStore._rawObjectStore.deleted) throw new o();
	if (e.objectStore.transaction._state !== "active") throw new u();
}, j = class {
	constructor(e, t) {
		this._rawIndex = t, this._name = t.name, this.objectStore = e, this.keyPath = ae(t.keyPath), this.multiEntry = t.multiEntry, this.unique = t.unique;
	}
	get name() {
		return this._name;
	}
	set name(e) {
		let t = this.objectStore.transaction;
		if (!t.db._runningVersionchangeTransaction) throw t._state === "active" ? new o() : new u();
		if (t._state !== "active") throw new u();
		if (this._rawIndex.deleted || this.objectStore._rawObjectStore.deleted) throw new o();
		if (e = String(e), e === this._name) return;
		if (this.objectStore.indexNames.contains(e)) throw new r();
		let n = this._name, i = [...this.objectStore.indexNames];
		this._name = e, this._rawIndex.name = e, this.objectStore._indexesCache.delete(n), this.objectStore._indexesCache.set(e, this), this.objectStore._rawObjectStore.rawIndexes.delete(n), this.objectStore._rawObjectStore.rawIndexes.set(e, this._rawIndex), this.objectStore.indexNames = new E(...Array.from(this.objectStore._rawObjectStore.rawIndexes.keys()).filter((e) => {
			let t = this.objectStore._rawObjectStore.rawIndexes.get(e);
			return t && !t.deleted;
		}).sort()), this.objectStore.transaction._createdIndexes.has(this._rawIndex) || t._rollbackLog.push(() => {
			this._name = n, this._rawIndex.name = n, this.objectStore._indexesCache.delete(e), this.objectStore._indexesCache.set(n, this), this.objectStore._rawObjectStore.rawIndexes.delete(e), this.objectStore._rawObjectStore.rawIndexes.set(n, this._rawIndex), this.objectStore.indexNames = new E(...i);
		});
	}
	openCursor(e, t) {
		A(this), e === null && (e = void 0), e !== void 0 && !(e instanceof _) && (e = _.only(h(e)));
		let n = new T();
		n.source = this, n.transaction = this.objectStore.transaction;
		let r = new S(this, e, t, n);
		return this.objectStore.transaction._execRequestAsync({
			operation: r._iterate.bind(r),
			request: n,
			source: this
		});
	}
	openKeyCursor(e, t) {
		A(this), e === null && (e = void 0), e !== void 0 && !(e instanceof _) && (e = _.only(h(e)));
		let n = new T();
		n.source = this, n.transaction = this.objectStore.transaction;
		let r = new x(this, e, t, n, !0);
		return this.objectStore.transaction._execRequestAsync({
			operation: r._iterate.bind(r),
			request: n,
			source: this
		});
	}
	get(e) {
		return A(this), e instanceof _ || (e = h(e)), this.objectStore.transaction._execRequestAsync({
			operation: this._rawIndex.getValue.bind(this._rawIndex, e),
			source: this
		});
	}
	getAll(e, t) {
		let n = k(e, t, arguments.length);
		A(this);
		let r = D(n.query);
		return this.objectStore.transaction._execRequestAsync({
			operation: this._rawIndex.getAllValues.bind(this._rawIndex, r, n.count, n.direction),
			source: this
		});
	}
	getKey(e) {
		return A(this), e instanceof _ || (e = h(e)), this.objectStore.transaction._execRequestAsync({
			operation: this._rawIndex.getKey.bind(this._rawIndex, e),
			source: this
		});
	}
	getAllKeys(e, t) {
		let n = k(e, t, arguments.length);
		A(this);
		let r = D(n.query);
		return this.objectStore.transaction._execRequestAsync({
			operation: this._rawIndex.getAllKeys.bind(this._rawIndex, r, n.count, n.direction),
			source: this
		});
	}
	getAllRecords(e) {
		let t, n, r;
		e !== void 0 && (e.query !== void 0 && (t = e.query), e.count !== void 0 && (n = O(e.count, "unsigned long")), e.direction !== void 0 && (r = e.direction)), A(this);
		let i = D(t);
		return this.objectStore.transaction._execRequestAsync({
			operation: this._rawIndex.getAllRecords.bind(this._rawIndex, i, n, r),
			source: this
		});
	}
	count(e) {
		return A(this), e === null && (e = void 0), e !== void 0 && !(e instanceof _) && (e = _.only(h(e))), this.objectStore.transaction._execRequestAsync({
			operation: () => this._rawIndex.count(e),
			source: this
		});
	}
	get [Symbol.toStringTag]() {
		return "IDBIndex";
	}
}, se = (e, t) => {
	if (Array.isArray(e)) throw Error("The key paths used in this section are always strings and never sequences, since it is not possible to create a object store which has a key generator and also has a key path that is a sequence.");
	let n = e.split(".");
	if (n.length === 0) throw Error("Assert: identifiers is not empty");
	n.pop();
	for (let e of n) {
		if (typeof t != "object" && !Array.isArray(t)) return !1;
		if (!Object.hasOwn(t, e)) return !0;
		t = t[e];
	}
	return typeof t == "object" || Array.isArray(t);
}, M = class {
	constructor(e, t, n) {
		this._key = e, this._primaryKey = t, this._value = n;
	}
	get key() {
		return this._key;
	}
	set key(e) {}
	get primaryKey() {
		return this._primaryKey;
	}
	set primaryKey(e) {}
	get value() {
		return this._value;
	}
	set value(e) {}
	get [Symbol.toStringTag]() {
		return "IDBRecord";
	}
}, ce = 2 / 3, le = new _(void 0, void 0, !1, !1), ue = class {
	_numTombstones = 0;
	_numNodes = 0;
	constructor(e) {
		this._keysAreUnique = !!e;
	}
	size() {
		return this._numNodes - this._numTombstones;
	}
	get(e) {
		return this._getByComparator(this._root, (t) => this._compare(e, t));
	}
	contains(e) {
		return !!this.get(e);
	}
	_compare(e, t) {
		let n = g(e.key, t.key);
		return n === 0 ? this._keysAreUnique ? 0 : g(e.value, t.value) : n;
	}
	_getByComparator(e, t) {
		let n = e;
		for (; n;) {
			let e = t(n.record);
			if (e < 0) n = n.left;
			else if (e > 0) n = n.right;
			else return n.record;
		}
	}
	put(e, t = !1) {
		if (!this._root) {
			this._root = {
				record: e,
				left: void 0,
				right: void 0,
				parent: void 0,
				deleted: !1,
				red: !1
			}, this._numNodes++;
			return;
		}
		return this._put(this._root, e, t);
	}
	_put(e, t, n) {
		let i = this._compare(t, e.record);
		if (i < 0) {
			if (e.left) return this._put(e.left, t, n);
			e.left = {
				record: t,
				left: void 0,
				right: void 0,
				parent: e,
				deleted: !1,
				red: !0
			}, this._onNewNodeInserted(e.left);
		} else if (i > 0) {
			if (e.right) return this._put(e.right, t, n);
			e.right = {
				record: t,
				left: void 0,
				right: void 0,
				parent: e,
				deleted: !1,
				red: !0
			}, this._onNewNodeInserted(e.right);
		} else if (e.deleted) e.deleted = !1, e.record = t, this._numTombstones--;
		else if (n) throw new r();
		else {
			let n = e.record;
			return e.record = t, n;
		}
	}
	delete(e) {
		if (this._root && (this._delete(this._root, e), this._numTombstones > this._numNodes * ce)) {
			let e = [...this.getAllRecords()];
			this._root = this._rebuild(e, void 0, !1), this._numNodes = e.length, this._numTombstones = 0;
		}
	}
	_delete(e, t) {
		if (!e) return;
		let n = this._compare(t, e.record);
		n < 0 ? this._delete(e.left, t) : n > 0 ? this._delete(e.right, t) : e.deleted ||= (this._numTombstones++, !0);
	}
	*getAllRecords(e = !1) {
		yield* this.getRecords(le, e);
	}
	*getRecords(e, t = !1) {
		yield* this._getRecordsForNode(this._root, e, t);
	}
	*_getRecordsForNode(e, t, n = !1) {
		e && (yield* this._findRecords(e, t, n));
	}
	*_findRecords(e, t, n = !1) {
		let { lower: r, upper: i, lowerOpen: a, upperOpen: o } = t, { record: { key: s } } = e, c = r === void 0 ? -1 : g(r, s), l = i === void 0 ? 1 : g(i, s), u = this._keysAreUnique ? c < 0 : c <= 0, d = this._keysAreUnique ? l > 0 : l >= 0, ee = n ? d : u, f = n ? u : d, p = n ? "right" : "left", m = n ? "left" : "right", h = a ? c < 0 : c <= 0, te = o ? l > 0 : l >= 0;
		ee && e[p] && (yield* this._findRecords(e[p], t, n)), h && te && !e.deleted && (yield e.record), f && e[m] && (yield* this._findRecords(e[m], t, n));
	}
	_onNewNodeInserted(e) {
		this._numNodes++, this._rebalanceTree(e);
	}
	_rebalanceTree(e) {
		let t = e.parent;
		do {
			if (!t.red) return;
			let n = t.parent;
			if (!n) {
				t.red = !1;
				return;
			}
			let r = t === n.right, i = r ? n.left : n.right;
			if (!i || !i.red) {
				e === (r ? t.left : t.right) && (this._rotateSubtree(t, r), e = t, t = r ? n.right : n.left), this._rotateSubtree(n, !r), t.red = !1, n.red = !0;
				return;
			}
			t.red = !1, i.red = !1, n.red = !0, e = n;
		} while (e.parent && (t = e.parent));
	}
	_rotateSubtree(e, t) {
		let n = e.parent, r = t ? e.left : e.right, i = t ? r.right : r.left;
		return e[t ? "left" : "right"] = i, i && (i.parent = e), r[t ? "right" : "left"] = e, r.parent = n, e.parent = r, n ? n[e === n.right ? "right" : "left"] = r : this._root = r, r;
	}
	_rebuild(e, t, n) {
		let { length: r } = e;
		if (!r) return;
		let i = r >>> 1, a = {
			record: e[i],
			left: void 0,
			right: void 0,
			parent: t,
			deleted: !1,
			red: n
		}, o = this._rebuild(e.slice(0, i), a, !n), s = this._rebuild(e.slice(i + 1), a, !n);
		return a.left = o, a.right = s, a;
	}
}, de = class {
	constructor(e) {
		this.keysAreUnique = e, this.records = new ue(this.keysAreUnique);
	}
	get(e) {
		let t = e instanceof _ ? e : _.only(e);
		return this.records.getRecords(t).next().value;
	}
	put(e, t = !1) {
		return this.records.put(e, t);
	}
	delete(e) {
		let t = e instanceof _ ? e : _.only(e), n = [...this.records.getRecords(t)];
		for (let e of n) this.records.delete(e);
		return n;
	}
	deleteByValue(e) {
		let t = e instanceof _ ? e : _.only(e), n = [];
		for (let e of this.records.getAllRecords()) t.includes(e.value) && (this.records.delete(e), n.push(e));
		return n;
	}
	clear() {
		let e = [...this.records.getAllRecords()];
		return this.records = new ue(this.keysAreUnique), e;
	}
	values(e, t = "next") {
		let n = t === "prev" || t === "prevunique", r = e ? this.records.getRecords(e, n) : this.records.getAllRecords(n);
		return { [Symbol.iterator]: () => {
			let e = () => r.next();
			if (t === "next" || t === "prev") return { next: e };
			if (t === "nextunique") {
				let t;
				return { next: () => {
					let n = e();
					for (; !n.done && t !== void 0 && g(t.key, n.value.key) === 0;) n = e();
					return t = n.value, n;
				} };
			}
			let n = e(), i = e();
			return { next: () => {
				for (; !i.done && g(n.value.key, i.value.key) === 0;) n = i, i = e();
				let t = n;
				return n = i, i = e(), t;
			} };
		} };
	}
	size() {
		return this.records.size();
	}
}, fe = class {
	deleted = !1;
	initialized = !1;
	constructor(e, t, n, r, i) {
		this.rawObjectStore = e, this.name = t, this.keyPath = n, this.multiEntry = r, this.unique = i, this.records = new de(i);
	}
	getKey(e) {
		let t = this.records.get(e);
		return t === void 0 ? void 0 : t.value;
	}
	getAllKeys(e, t, n) {
		(t === void 0 || t === 0) && (t = Infinity);
		let r = [];
		for (let i of this.records.values(e, n)) if (r.push(structuredClone(i.value)), r.length >= t) break;
		return r;
	}
	getValue(e) {
		let t = this.records.get(e);
		return t === void 0 ? void 0 : this.rawObjectStore.getValue(t.value);
	}
	getAllValues(e, t, n) {
		(t === void 0 || t === 0) && (t = Infinity);
		let r = [];
		for (let i of this.records.values(e, n)) if (r.push(this.rawObjectStore.getValue(i.value)), r.length >= t) break;
		return r;
	}
	getAllRecords(e, t, n) {
		(t === void 0 || t === 0) && (t = Infinity);
		let r = [];
		for (let i of this.records.values(e, n)) if (r.push(new M(structuredClone(i.key), structuredClone(this.rawObjectStore.getKey(i.value)), this.rawObjectStore.getValue(i.value))), r.length >= t) break;
		return r;
	}
	storeRecord(e) {
		let t;
		try {
			t = v(this.keyPath, e.value).key;
		} catch (e) {
			if (e.name === "DataError") return;
			throw e;
		}
		if (!this.multiEntry || !Array.isArray(t)) try {
			h(t);
		} catch {
			return;
		}
		else {
			let e = [];
			for (let n of t) if (e.indexOf(n) < 0) try {
				e.push(h(n));
			} catch {}
			t = e;
		}
		if (!this.multiEntry || !Array.isArray(t)) {
			if (this.unique && this.records.get(t)) throw new r();
		} else if (this.unique) {
			for (let e of t) if (this.records.get(e)) throw new r();
		}
		if (!this.multiEntry || !Array.isArray(t)) this.records.put({
			key: t,
			value: e.key
		});
		else for (let n of t) this.records.put({
			key: n,
			value: e.key
		});
	}
	initialize(e) {
		if (this.initialized) throw Error("Index already initialized");
		e._execRequestAsync({
			operation: () => {
				try {
					for (let e of this.rawObjectStore.records.values()) this.storeRecord(e);
					this.initialized = !0;
				} catch (t) {
					e._abort(t.name);
				}
			},
			source: null
		});
	}
	count(e) {
		let t = 0;
		for (let n of this.records.values(e)) t += 1;
		return t;
	}
}, N = (e, t) => {
	if (e != null && typeof e != "string" && e.toString && (t === "array" || !Array.isArray(e)) && (e = e.toString()), typeof e == "string") {
		if (e === "" && t !== "string") return;
		try {
			if (e.length >= 1 && /^(?:[$A-Z_a-z\xAA\xB5\xBA\xC0-\xD6\xD8-\xF6\xF8-\u02C1\u02C6-\u02D1\u02E0-\u02E4\u02EC\u02EE\u0370-\u0374\u0376\u0377\u037A-\u037D\u037F\u0386\u0388-\u038A\u038C\u038E-\u03A1\u03A3-\u03F5\u03F7-\u0481\u048A-\u052F\u0531-\u0556\u0559\u0561-\u0587\u05D0-\u05EA\u05F0-\u05F2\u0620-\u064A\u066E\u066F\u0671-\u06D3\u06D5\u06E5\u06E6\u06EE\u06EF\u06FA-\u06FC\u06FF\u0710\u0712-\u072F\u074D-\u07A5\u07B1\u07CA-\u07EA\u07F4\u07F5\u07FA\u0800-\u0815\u081A\u0824\u0828\u0840-\u0858\u08A0-\u08B2\u0904-\u0939\u093D\u0950\u0958-\u0961\u0971-\u0980\u0985-\u098C\u098F\u0990\u0993-\u09A8\u09AA-\u09B0\u09B2\u09B6-\u09B9\u09BD\u09CE\u09DC\u09DD\u09DF-\u09E1\u09F0\u09F1\u0A05-\u0A0A\u0A0F\u0A10\u0A13-\u0A28\u0A2A-\u0A30\u0A32\u0A33\u0A35\u0A36\u0A38\u0A39\u0A59-\u0A5C\u0A5E\u0A72-\u0A74\u0A85-\u0A8D\u0A8F-\u0A91\u0A93-\u0AA8\u0AAA-\u0AB0\u0AB2\u0AB3\u0AB5-\u0AB9\u0ABD\u0AD0\u0AE0\u0AE1\u0B05-\u0B0C\u0B0F\u0B10\u0B13-\u0B28\u0B2A-\u0B30\u0B32\u0B33\u0B35-\u0B39\u0B3D\u0B5C\u0B5D\u0B5F-\u0B61\u0B71\u0B83\u0B85-\u0B8A\u0B8E-\u0B90\u0B92-\u0B95\u0B99\u0B9A\u0B9C\u0B9E\u0B9F\u0BA3\u0BA4\u0BA8-\u0BAA\u0BAE-\u0BB9\u0BD0\u0C05-\u0C0C\u0C0E-\u0C10\u0C12-\u0C28\u0C2A-\u0C39\u0C3D\u0C58\u0C59\u0C60\u0C61\u0C85-\u0C8C\u0C8E-\u0C90\u0C92-\u0CA8\u0CAA-\u0CB3\u0CB5-\u0CB9\u0CBD\u0CDE\u0CE0\u0CE1\u0CF1\u0CF2\u0D05-\u0D0C\u0D0E-\u0D10\u0D12-\u0D3A\u0D3D\u0D4E\u0D60\u0D61\u0D7A-\u0D7F\u0D85-\u0D96\u0D9A-\u0DB1\u0DB3-\u0DBB\u0DBD\u0DC0-\u0DC6\u0E01-\u0E30\u0E32\u0E33\u0E40-\u0E46\u0E81\u0E82\u0E84\u0E87\u0E88\u0E8A\u0E8D\u0E94-\u0E97\u0E99-\u0E9F\u0EA1-\u0EA3\u0EA5\u0EA7\u0EAA\u0EAB\u0EAD-\u0EB0\u0EB2\u0EB3\u0EBD\u0EC0-\u0EC4\u0EC6\u0EDC-\u0EDF\u0F00\u0F40-\u0F47\u0F49-\u0F6C\u0F88-\u0F8C\u1000-\u102A\u103F\u1050-\u1055\u105A-\u105D\u1061\u1065\u1066\u106E-\u1070\u1075-\u1081\u108E\u10A0-\u10C5\u10C7\u10CD\u10D0-\u10FA\u10FC-\u1248\u124A-\u124D\u1250-\u1256\u1258\u125A-\u125D\u1260-\u1288\u128A-\u128D\u1290-\u12B0\u12B2-\u12B5\u12B8-\u12BE\u12C0\u12C2-\u12C5\u12C8-\u12D6\u12D8-\u1310\u1312-\u1315\u1318-\u135A\u1380-\u138F\u13A0-\u13F4\u1401-\u166C\u166F-\u167F\u1681-\u169A\u16A0-\u16EA\u16EE-\u16F8\u1700-\u170C\u170E-\u1711\u1720-\u1731\u1740-\u1751\u1760-\u176C\u176E-\u1770\u1780-\u17B3\u17D7\u17DC\u1820-\u1877\u1880-\u18A8\u18AA\u18B0-\u18F5\u1900-\u191E\u1950-\u196D\u1970-\u1974\u1980-\u19AB\u19C1-\u19C7\u1A00-\u1A16\u1A20-\u1A54\u1AA7\u1B05-\u1B33\u1B45-\u1B4B\u1B83-\u1BA0\u1BAE\u1BAF\u1BBA-\u1BE5\u1C00-\u1C23\u1C4D-\u1C4F\u1C5A-\u1C7D\u1CE9-\u1CEC\u1CEE-\u1CF1\u1CF5\u1CF6\u1D00-\u1DBF\u1E00-\u1F15\u1F18-\u1F1D\u1F20-\u1F45\u1F48-\u1F4D\u1F50-\u1F57\u1F59\u1F5B\u1F5D\u1F5F-\u1F7D\u1F80-\u1FB4\u1FB6-\u1FBC\u1FBE\u1FC2-\u1FC4\u1FC6-\u1FCC\u1FD0-\u1FD3\u1FD6-\u1FDB\u1FE0-\u1FEC\u1FF2-\u1FF4\u1FF6-\u1FFC\u2071\u207F\u2090-\u209C\u2102\u2107\u210A-\u2113\u2115\u2119-\u211D\u2124\u2126\u2128\u212A-\u212D\u212F-\u2139\u213C-\u213F\u2145-\u2149\u214E\u2160-\u2188\u2C00-\u2C2E\u2C30-\u2C5E\u2C60-\u2CE4\u2CEB-\u2CEE\u2CF2\u2CF3\u2D00-\u2D25\u2D27\u2D2D\u2D30-\u2D67\u2D6F\u2D80-\u2D96\u2DA0-\u2DA6\u2DA8-\u2DAE\u2DB0-\u2DB6\u2DB8-\u2DBE\u2DC0-\u2DC6\u2DC8-\u2DCE\u2DD0-\u2DD6\u2DD8-\u2DDE\u2E2F\u3005-\u3007\u3021-\u3029\u3031-\u3035\u3038-\u303C\u3041-\u3096\u309D-\u309F\u30A1-\u30FA\u30FC-\u30FF\u3105-\u312D\u3131-\u318E\u31A0-\u31BA\u31F0-\u31FF\u3400-\u4DB5\u4E00-\u9FCC\uA000-\uA48C\uA4D0-\uA4FD\uA500-\uA60C\uA610-\uA61F\uA62A\uA62B\uA640-\uA66E\uA67F-\uA69D\uA6A0-\uA6EF\uA717-\uA71F\uA722-\uA788\uA78B-\uA78E\uA790-\uA7AD\uA7B0\uA7B1\uA7F7-\uA801\uA803-\uA805\uA807-\uA80A\uA80C-\uA822\uA840-\uA873\uA882-\uA8B3\uA8F2-\uA8F7\uA8FB\uA90A-\uA925\uA930-\uA946\uA960-\uA97C\uA984-\uA9B2\uA9CF\uA9E0-\uA9E4\uA9E6-\uA9EF\uA9FA-\uA9FE\uAA00-\uAA28\uAA40-\uAA42\uAA44-\uAA4B\uAA60-\uAA76\uAA7A\uAA7E-\uAAAF\uAAB1\uAAB5\uAAB6\uAAB9-\uAABD\uAAC0\uAAC2\uAADB-\uAADD\uAAE0-\uAAEA\uAAF2-\uAAF4\uAB01-\uAB06\uAB09-\uAB0E\uAB11-\uAB16\uAB20-\uAB26\uAB28-\uAB2E\uAB30-\uAB5A\uAB5C-\uAB5F\uAB64\uAB65\uABC0-\uABE2\uAC00-\uD7A3\uD7B0-\uD7C6\uD7CB-\uD7FB\uF900-\uFA6D\uFA70-\uFAD9\uFB00-\uFB06\uFB13-\uFB17\uFB1D\uFB1F-\uFB28\uFB2A-\uFB36\uFB38-\uFB3C\uFB3E\uFB40\uFB41\uFB43\uFB44\uFB46-\uFBB1\uFBD3-\uFD3D\uFD50-\uFD8F\uFD92-\uFDC7\uFDF0-\uFDFB\uFE70-\uFE74\uFE76-\uFEFC\uFF21-\uFF3A\uFF41-\uFF5A\uFF66-\uFFBE\uFFC2-\uFFC7\uFFCA-\uFFCF\uFFD2-\uFFD7\uFFDA-\uFFDC])(?:[$0-9A-Z_a-z\xAA\xB5\xBA\xC0-\xD6\xD8-\xF6\xF8-\u02C1\u02C6-\u02D1\u02E0-\u02E4\u02EC\u02EE\u0300-\u0374\u0376\u0377\u037A-\u037D\u037F\u0386\u0388-\u038A\u038C\u038E-\u03A1\u03A3-\u03F5\u03F7-\u0481\u0483-\u0487\u048A-\u052F\u0531-\u0556\u0559\u0561-\u0587\u0591-\u05BD\u05BF\u05C1\u05C2\u05C4\u05C5\u05C7\u05D0-\u05EA\u05F0-\u05F2\u0610-\u061A\u0620-\u0669\u066E-\u06D3\u06D5-\u06DC\u06DF-\u06E8\u06EA-\u06FC\u06FF\u0710-\u074A\u074D-\u07B1\u07C0-\u07F5\u07FA\u0800-\u082D\u0840-\u085B\u08A0-\u08B2\u08E4-\u0963\u0966-\u096F\u0971-\u0983\u0985-\u098C\u098F\u0990\u0993-\u09A8\u09AA-\u09B0\u09B2\u09B6-\u09B9\u09BC-\u09C4\u09C7\u09C8\u09CB-\u09CE\u09D7\u09DC\u09DD\u09DF-\u09E3\u09E6-\u09F1\u0A01-\u0A03\u0A05-\u0A0A\u0A0F\u0A10\u0A13-\u0A28\u0A2A-\u0A30\u0A32\u0A33\u0A35\u0A36\u0A38\u0A39\u0A3C\u0A3E-\u0A42\u0A47\u0A48\u0A4B-\u0A4D\u0A51\u0A59-\u0A5C\u0A5E\u0A66-\u0A75\u0A81-\u0A83\u0A85-\u0A8D\u0A8F-\u0A91\u0A93-\u0AA8\u0AAA-\u0AB0\u0AB2\u0AB3\u0AB5-\u0AB9\u0ABC-\u0AC5\u0AC7-\u0AC9\u0ACB-\u0ACD\u0AD0\u0AE0-\u0AE3\u0AE6-\u0AEF\u0B01-\u0B03\u0B05-\u0B0C\u0B0F\u0B10\u0B13-\u0B28\u0B2A-\u0B30\u0B32\u0B33\u0B35-\u0B39\u0B3C-\u0B44\u0B47\u0B48\u0B4B-\u0B4D\u0B56\u0B57\u0B5C\u0B5D\u0B5F-\u0B63\u0B66-\u0B6F\u0B71\u0B82\u0B83\u0B85-\u0B8A\u0B8E-\u0B90\u0B92-\u0B95\u0B99\u0B9A\u0B9C\u0B9E\u0B9F\u0BA3\u0BA4\u0BA8-\u0BAA\u0BAE-\u0BB9\u0BBE-\u0BC2\u0BC6-\u0BC8\u0BCA-\u0BCD\u0BD0\u0BD7\u0BE6-\u0BEF\u0C00-\u0C03\u0C05-\u0C0C\u0C0E-\u0C10\u0C12-\u0C28\u0C2A-\u0C39\u0C3D-\u0C44\u0C46-\u0C48\u0C4A-\u0C4D\u0C55\u0C56\u0C58\u0C59\u0C60-\u0C63\u0C66-\u0C6F\u0C81-\u0C83\u0C85-\u0C8C\u0C8E-\u0C90\u0C92-\u0CA8\u0CAA-\u0CB3\u0CB5-\u0CB9\u0CBC-\u0CC4\u0CC6-\u0CC8\u0CCA-\u0CCD\u0CD5\u0CD6\u0CDE\u0CE0-\u0CE3\u0CE6-\u0CEF\u0CF1\u0CF2\u0D01-\u0D03\u0D05-\u0D0C\u0D0E-\u0D10\u0D12-\u0D3A\u0D3D-\u0D44\u0D46-\u0D48\u0D4A-\u0D4E\u0D57\u0D60-\u0D63\u0D66-\u0D6F\u0D7A-\u0D7F\u0D82\u0D83\u0D85-\u0D96\u0D9A-\u0DB1\u0DB3-\u0DBB\u0DBD\u0DC0-\u0DC6\u0DCA\u0DCF-\u0DD4\u0DD6\u0DD8-\u0DDF\u0DE6-\u0DEF\u0DF2\u0DF3\u0E01-\u0E3A\u0E40-\u0E4E\u0E50-\u0E59\u0E81\u0E82\u0E84\u0E87\u0E88\u0E8A\u0E8D\u0E94-\u0E97\u0E99-\u0E9F\u0EA1-\u0EA3\u0EA5\u0EA7\u0EAA\u0EAB\u0EAD-\u0EB9\u0EBB-\u0EBD\u0EC0-\u0EC4\u0EC6\u0EC8-\u0ECD\u0ED0-\u0ED9\u0EDC-\u0EDF\u0F00\u0F18\u0F19\u0F20-\u0F29\u0F35\u0F37\u0F39\u0F3E-\u0F47\u0F49-\u0F6C\u0F71-\u0F84\u0F86-\u0F97\u0F99-\u0FBC\u0FC6\u1000-\u1049\u1050-\u109D\u10A0-\u10C5\u10C7\u10CD\u10D0-\u10FA\u10FC-\u1248\u124A-\u124D\u1250-\u1256\u1258\u125A-\u125D\u1260-\u1288\u128A-\u128D\u1290-\u12B0\u12B2-\u12B5\u12B8-\u12BE\u12C0\u12C2-\u12C5\u12C8-\u12D6\u12D8-\u1310\u1312-\u1315\u1318-\u135A\u135D-\u135F\u1380-\u138F\u13A0-\u13F4\u1401-\u166C\u166F-\u167F\u1681-\u169A\u16A0-\u16EA\u16EE-\u16F8\u1700-\u170C\u170E-\u1714\u1720-\u1734\u1740-\u1753\u1760-\u176C\u176E-\u1770\u1772\u1773\u1780-\u17D3\u17D7\u17DC\u17DD\u17E0-\u17E9\u180B-\u180D\u1810-\u1819\u1820-\u1877\u1880-\u18AA\u18B0-\u18F5\u1900-\u191E\u1920-\u192B\u1930-\u193B\u1946-\u196D\u1970-\u1974\u1980-\u19AB\u19B0-\u19C9\u19D0-\u19D9\u1A00-\u1A1B\u1A20-\u1A5E\u1A60-\u1A7C\u1A7F-\u1A89\u1A90-\u1A99\u1AA7\u1AB0-\u1ABD\u1B00-\u1B4B\u1B50-\u1B59\u1B6B-\u1B73\u1B80-\u1BF3\u1C00-\u1C37\u1C40-\u1C49\u1C4D-\u1C7D\u1CD0-\u1CD2\u1CD4-\u1CF6\u1CF8\u1CF9\u1D00-\u1DF5\u1DFC-\u1F15\u1F18-\u1F1D\u1F20-\u1F45\u1F48-\u1F4D\u1F50-\u1F57\u1F59\u1F5B\u1F5D\u1F5F-\u1F7D\u1F80-\u1FB4\u1FB6-\u1FBC\u1FBE\u1FC2-\u1FC4\u1FC6-\u1FCC\u1FD0-\u1FD3\u1FD6-\u1FDB\u1FE0-\u1FEC\u1FF2-\u1FF4\u1FF6-\u1FFC\u200C\u200D\u203F\u2040\u2054\u2071\u207F\u2090-\u209C\u20D0-\u20DC\u20E1\u20E5-\u20F0\u2102\u2107\u210A-\u2113\u2115\u2119-\u211D\u2124\u2126\u2128\u212A-\u212D\u212F-\u2139\u213C-\u213F\u2145-\u2149\u214E\u2160-\u2188\u2C00-\u2C2E\u2C30-\u2C5E\u2C60-\u2CE4\u2CEB-\u2CF3\u2D00-\u2D25\u2D27\u2D2D\u2D30-\u2D67\u2D6F\u2D7F-\u2D96\u2DA0-\u2DA6\u2DA8-\u2DAE\u2DB0-\u2DB6\u2DB8-\u2DBE\u2DC0-\u2DC6\u2DC8-\u2DCE\u2DD0-\u2DD6\u2DD8-\u2DDE\u2DE0-\u2DFF\u2E2F\u3005-\u3007\u3021-\u302F\u3031-\u3035\u3038-\u303C\u3041-\u3096\u3099\u309A\u309D-\u309F\u30A1-\u30FA\u30FC-\u30FF\u3105-\u312D\u3131-\u318E\u31A0-\u31BA\u31F0-\u31FF\u3400-\u4DB5\u4E00-\u9FCC\uA000-\uA48C\uA4D0-\uA4FD\uA500-\uA60C\uA610-\uA62B\uA640-\uA66F\uA674-\uA67D\uA67F-\uA69D\uA69F-\uA6F1\uA717-\uA71F\uA722-\uA788\uA78B-\uA78E\uA790-\uA7AD\uA7B0\uA7B1\uA7F7-\uA827\uA840-\uA873\uA880-\uA8C4\uA8D0-\uA8D9\uA8E0-\uA8F7\uA8FB\uA900-\uA92D\uA930-\uA953\uA960-\uA97C\uA980-\uA9C0\uA9CF-\uA9D9\uA9E0-\uA9FE\uAA00-\uAA36\uAA40-\uAA4D\uAA50-\uAA59\uAA60-\uAA76\uAA7A-\uAAC2\uAADB-\uAADD\uAAE0-\uAAEF\uAAF2-\uAAF6\uAB01-\uAB06\uAB09-\uAB0E\uAB11-\uAB16\uAB20-\uAB26\uAB28-\uAB2E\uAB30-\uAB5A\uAB5C-\uAB5F\uAB64\uAB65\uABC0-\uABEA\uABEC\uABED\uABF0-\uABF9\uAC00-\uD7A3\uD7B0-\uD7C6\uD7CB-\uD7FB\uF900-\uFA6D\uFA70-\uFAD9\uFB00-\uFB06\uFB13-\uFB17\uFB1D-\uFB28\uFB2A-\uFB36\uFB38-\uFB3C\uFB3E\uFB40\uFB41\uFB43\uFB44\uFB46-\uFBB1\uFBD3-\uFD3D\uFD50-\uFD8F\uFD92-\uFDC7\uFDF0-\uFDFB\uFE00-\uFE0F\uFE20-\uFE2D\uFE33\uFE34\uFE4D-\uFE4F\uFE70-\uFE74\uFE76-\uFEFC\uFF10-\uFF19\uFF21-\uFF3A\uFF3F\uFF41-\uFF5A\uFF66-\uFFBE\uFFC2-\uFFC7\uFFCA-\uFFCF\uFFD2-\uFFD7\uFFDA-\uFFDC])*$/.test(e)) return;
		} catch (e) {
			throw new l(e.message);
		}
		if (e.indexOf(" ") >= 0) throw new l("The keypath argument contains an invalid key path (no spaces allowed).");
	}
	if (Array.isArray(e) && e.length > 0) {
		if (t) throw new l("The keypath argument contains an invalid key path (nested arrays).");
		for (let t of e) N(t, "array");
		return;
	} else if (typeof e == "string" && e.indexOf(".") >= 0) {
		e = e.split(".");
		for (let t of e) N(t, "string");
		return;
	}
	throw new l();
}, P = (e) => {
	if (e._rawObjectStore.deleted) throw new o();
	if (e.transaction._state !== "active") throw new u();
}, pe = (e, t, n) => {
	if (P(e), e.transaction.mode === "readonly") throw new c();
	if (e.keyPath !== null && n !== void 0) throw new i();
	let r = ne(t, e.transaction);
	if (e.keyPath !== null) {
		let t = v(e.keyPath, r);
		if (t.type === "found") h(t.key);
		else if (!e._rawObjectStore.keyGenerator) throw new i();
		else if (!se(e.keyPath, r)) throw new i();
	}
	if (e.keyPath === null && e._rawObjectStore.keyGenerator === null && n === void 0) throw new i();
	return n !== void 0 && (n = h(n)), {
		key: n,
		value: r
	};
}, F = class {
	_indexesCache = /* @__PURE__ */ new Map();
	constructor(e, t) {
		this._rawObjectStore = t, this._name = t.name, this.keyPath = ae(t.keyPath), this.autoIncrement = t.autoIncrement, this.transaction = e, this.indexNames = new E(...Array.from(t.rawIndexes.keys()).sort());
	}
	get name() {
		return this._name;
	}
	set name(e) {
		let t = this.transaction;
		if (!t.db._runningVersionchangeTransaction) throw t._state === "active" ? new o() : new u();
		if (P(this), e = String(e), e === this._name) return;
		if (this._rawObjectStore.rawDatabase.rawObjectStores.has(e)) throw new r();
		let n = this._name, i = [...t.db.objectStoreNames];
		this._name = e, this._rawObjectStore.name = e, this.transaction._objectStoresCache.delete(n), this.transaction._objectStoresCache.set(e, this), this._rawObjectStore.rawDatabase.rawObjectStores.delete(n), this._rawObjectStore.rawDatabase.rawObjectStores.set(e, this._rawObjectStore), t.db.objectStoreNames = new E(...Array.from(this._rawObjectStore.rawDatabase.rawObjectStores.keys()).filter((e) => {
			let t = this._rawObjectStore.rawDatabase.rawObjectStores.get(e);
			return t && !t.deleted;
		}).sort());
		let a = new Set(t._scope), s = [...t.objectStoreNames];
		this.transaction._scope.delete(n), t._scope.add(e), t.objectStoreNames = new E(...Array.from(t._scope).sort()), this.transaction._createdObjectStores.has(this._rawObjectStore) || t._rollbackLog.push(() => {
			this._name = n, this._rawObjectStore.name = n, this.transaction._objectStoresCache.delete(e), this.transaction._objectStoresCache.set(n, this), this._rawObjectStore.rawDatabase.rawObjectStores.delete(e), this._rawObjectStore.rawDatabase.rawObjectStores.set(n, this._rawObjectStore), t.db.objectStoreNames = new E(...i), t._scope = a, t.objectStoreNames = new E(...s);
		});
	}
	put(e, t) {
		if (arguments.length === 0) throw TypeError();
		let n = pe(this, e, t);
		return this.transaction._execRequestAsync({
			operation: this._rawObjectStore.storeRecord.bind(this._rawObjectStore, n, !1, this.transaction._rollbackLog),
			source: this
		});
	}
	add(e, t) {
		if (arguments.length === 0) throw TypeError();
		let n = pe(this, e, t);
		return this.transaction._execRequestAsync({
			operation: this._rawObjectStore.storeRecord.bind(this._rawObjectStore, n, !0, this.transaction._rollbackLog),
			source: this
		});
	}
	delete(e) {
		if (arguments.length === 0) throw TypeError();
		if (P(this), this.transaction.mode === "readonly") throw new c();
		return e instanceof _ || (e = h(e)), this.transaction._execRequestAsync({
			operation: this._rawObjectStore.deleteRecord.bind(this._rawObjectStore, e, this.transaction._rollbackLog),
			source: this
		});
	}
	get(e) {
		if (arguments.length === 0) throw TypeError();
		return P(this), e instanceof _ || (e = h(e)), this.transaction._execRequestAsync({
			operation: this._rawObjectStore.getValue.bind(this._rawObjectStore, e),
			source: this
		});
	}
	getAll(e, t) {
		let n = k(e, t, arguments.length);
		P(this);
		let r = D(n.query);
		return this.transaction._execRequestAsync({
			operation: this._rawObjectStore.getAllValues.bind(this._rawObjectStore, r, n.count, n.direction),
			source: this
		});
	}
	getKey(e) {
		if (arguments.length === 0) throw TypeError();
		return P(this), e instanceof _ || (e = h(e)), this.transaction._execRequestAsync({
			operation: this._rawObjectStore.getKey.bind(this._rawObjectStore, e),
			source: this
		});
	}
	getAllKeys(e, t) {
		let n = k(e, t, arguments.length);
		P(this);
		let r = D(n.query);
		return this.transaction._execRequestAsync({
			operation: this._rawObjectStore.getAllKeys.bind(this._rawObjectStore, r, n.count, n.direction),
			source: this
		});
	}
	getAllRecords(e) {
		let t, n, r;
		e !== void 0 && (e.query !== void 0 && (t = e.query), e.count !== void 0 && (n = O(e.count, "unsigned long")), e.direction !== void 0 && (r = e.direction)), P(this);
		let i = D(t);
		return this.transaction._execRequestAsync({
			operation: this._rawObjectStore.getAllRecords.bind(this._rawObjectStore, i, n, r),
			source: this
		});
	}
	clear() {
		if (P(this), this.transaction.mode === "readonly") throw new c();
		return this.transaction._execRequestAsync({
			operation: this._rawObjectStore.clear.bind(this._rawObjectStore, this.transaction._rollbackLog),
			source: this
		});
	}
	openCursor(e, t) {
		P(this), e === null && (e = void 0), e !== void 0 && !(e instanceof _) && (e = _.only(h(e)));
		let n = new T();
		n.source = this, n.transaction = this.transaction;
		let r = new S(this, e, t, n);
		return this.transaction._execRequestAsync({
			operation: r._iterate.bind(r),
			request: n,
			source: this
		});
	}
	openKeyCursor(e, t) {
		P(this), e === null && (e = void 0), e !== void 0 && !(e instanceof _) && (e = _.only(h(e)));
		let n = new T();
		n.source = this, n.transaction = this.transaction;
		let r = new x(this, e, t, n, !0);
		return this.transaction._execRequestAsync({
			operation: r._iterate.bind(r),
			request: n,
			source: this
		});
	}
	createIndex(e, t, n = {}) {
		if (arguments.length < 2) throw TypeError();
		let i = n.multiEntry !== void 0 && n.multiEntry, s = n.unique !== void 0 && n.unique;
		if (this.transaction.mode !== "versionchange") throw new o();
		if (P(this), this.indexNames.contains(e)) throw new r();
		if (N(t), Array.isArray(t) && i) throw new a();
		let c = [...this.indexNames], l = new fe(this._rawObjectStore, e, t, i, s);
		return this.indexNames._push(e), this.indexNames._sort(), this.transaction._createdIndexes.add(l), this._rawObjectStore.rawIndexes.set(e, l), l.initialize(this.transaction), this.transaction._rollbackLog.push(() => {
			l.deleted = !0, this.indexNames = new E(...c), this._rawObjectStore.rawIndexes.delete(l.name);
		}), new j(this, l);
	}
	index(e) {
		if (arguments.length === 0) throw TypeError();
		if (this._rawObjectStore.deleted || this.transaction._state === "finished") throw new o();
		let t = this._indexesCache.get(e);
		if (t !== void 0) return t;
		let n = this._rawObjectStore.rawIndexes.get(e);
		if (!this.indexNames.contains(e) || n === void 0) throw new s();
		let r = new j(this, n);
		return this._indexesCache.set(e, r), r;
	}
	deleteIndex(e) {
		if (arguments.length === 0) throw TypeError();
		if (this.transaction.mode !== "versionchange") throw new o();
		P(this);
		let t = this._rawObjectStore.rawIndexes.get(e);
		if (t === void 0) throw new s();
		this.transaction._rollbackLog.push(() => {
			t.deleted = !1, this._rawObjectStore.rawIndexes.set(t.name, t), this.indexNames._push(t.name), this.indexNames._sort();
		}), this.indexNames = new E(...Array.from(this.indexNames).filter((t) => t !== e)), t.deleted = !0, this.transaction._execRequestAsync({
			operation: () => {
				let n = this._rawObjectStore.rawIndexes.get(e);
				t === n && this._rawObjectStore.rawIndexes.delete(e);
			},
			source: this
		});
	}
	count(e) {
		return P(this), e === null && (e = void 0), e !== void 0 && !(e instanceof _) && (e = _.only(h(e))), this.transaction._execRequestAsync({
			operation: () => this._rawObjectStore.count(e),
			source: this
		});
	}
	get [Symbol.toStringTag]() {
		return "IDBObjectStore";
	}
}, I = class {
	eventPath = [];
	NONE = 0;
	CAPTURING_PHASE = 1;
	AT_TARGET = 2;
	BUBBLING_PHASE = 3;
	propagationStopped = !1;
	immediatePropagationStopped = !1;
	canceled = !1;
	initialized = !0;
	dispatched = !1;
	target = null;
	currentTarget = null;
	eventPhase = 0;
	defaultPrevented = !1;
	isTrusted = !1;
	timeStamp = Date.now();
	constructor(e, t = {}) {
		this.type = e, this.bubbles = t.bubbles !== void 0 && t.bubbles, this.cancelable = t.cancelable !== void 0 && t.cancelable;
	}
	preventDefault() {
		this.cancelable && (this.canceled = !0);
	}
	stopPropagation() {
		this.propagationStopped = !0;
	}
	stopImmediatePropagation() {
		this.propagationStopped = !0, this.immediatePropagationStopped = !0;
	}
};
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/scheduling.js
function me() {
	if (typeof navigator < "u" && /jsdom/.test(navigator.userAgent)) {
		let e = Node.constructor;
		return new e("return setImmediate")();
	} else return;
}
var he = typeof scheduler < "u" && ((e) => scheduler.postTask(e)), ge = (e) => setTimeout(e, 0), L = (e) => {
	(globalThis.setImmediate || me() || he || ge)(e);
}, _e = class extends w {
	_state = "active";
	_started = !1;
	_rollbackLog = [];
	_objectStoresCache = /* @__PURE__ */ new Map();
	_openRequest = null;
	error = null;
	onabort = null;
	oncomplete = null;
	onerror = null;
	_requests = [];
	_createdIndexes = /* @__PURE__ */ new Set();
	_createdObjectStores = /* @__PURE__ */ new Set();
	constructor(e, t, n, r) {
		super(), this._scope = new Set(e), this.mode = t, this.durability = n, this.db = r, this.objectStoreNames = new E(...Array.from(this._scope).sort());
	}
	_abort(e) {
		for (let e of this._rollbackLog.reverse()) e();
		if (e !== null) {
			let t = new DOMException(void 0, e);
			this.error = t;
		}
		for (let { request: e } of this._requests) e.readyState !== "done" && (e.readyState = "done", e.source && L(() => {
			e.result = void 0, e.error = new n();
			let t = new I("error", {
				bubbles: !0,
				cancelable: !0
			});
			t.eventPath = [this.db, this];
			try {
				e.dispatchEvent(t);
			} catch {
				this._state === "active" && this._abort("AbortError");
			}
		}));
		L(() => {
			let e = this.mode === "versionchange";
			e && (this.db._rawDatabase.connections = this.db._rawDatabase.connections.filter((e) => !e._rawDatabase.transactions.includes(this)));
			let t = new I("abort", {
				bubbles: !0,
				cancelable: !1
			});
			if (t.eventPath = [this.db], this.dispatchEvent(t), e) {
				let e = this._openRequest;
				e.transaction = null, e.result = void 0;
			}
		}), this._state = "finished";
	}
	abort() {
		if (this._state === "committing" || this._state === "finished") throw new o();
		this._state = "active", this._abort(null);
	}
	objectStore(e) {
		if (this._state !== "active") throw new o();
		let t = this._objectStoresCache.get(e);
		if (t !== void 0) return t;
		let n = this.db._rawDatabase.rawObjectStores.get(e);
		if (!this._scope.has(e) || n === void 0) throw new s();
		let r = new F(this, n);
		return this._objectStoresCache.set(e, r), r;
	}
	_execRequestAsync(e) {
		let t = e.source, n = e.operation, r = Object.hasOwn(e, "request") ? e.request : null;
		if (this._state !== "active") throw new u();
		return r || (t ? (r = new T(), r.source = t, r.transaction = t.transaction) : r = new T()), this._requests.push({
			operation: n,
			request: r
		}), r;
	}
	_start() {
		this._started = !0;
		let e, t;
		for (; this._requests.length > 0;) {
			let n = this._requests.shift();
			if (n && n.request.readyState !== "done") {
				t = n.request, e = n.operation;
				break;
			}
		}
		if (t && e) {
			if (!t.source) e();
			else {
				let n, r;
				try {
					let n = e();
					t.readyState = "done", t.result = n, t.error = void 0, this._state === "inactive" && (this._state = "active"), r = new I("success", {
						bubbles: !1,
						cancelable: !1
					});
				} catch (e) {
					t.readyState = "done", t.result = void 0, t.error = e, this._state === "inactive" && (this._state = "active"), r = new I("error", {
						bubbles: !0,
						cancelable: !0
					}), n = this._abort.bind(this, e.name);
				}
				try {
					r.eventPath = [this.db, this], t.dispatchEvent(r);
				} catch {
					this._state === "active" && (this._abort("AbortError"), n = void 0);
				}
				r.canceled || n && n();
			}
			L(this._start.bind(this));
			return;
		}
		if (this._state !== "finished" && (this._state = "finished", !this.error)) {
			let e = new I("complete");
			this.dispatchEvent(e);
		}
	}
	commit() {
		if (this._state !== "active") throw new o();
		this._state = "committing";
	}
	get [Symbol.toStringTag]() {
		return "IDBTransaction";
	}
}, ve = 9007199254740992, ye = class {
	num = 0;
	next() {
		if (this.num >= ve) throw new r();
		return this.num += 1, this.num;
	}
	setIfLarger(e) {
		let t = Math.floor(Math.min(e, ve)) - 1;
		t >= this.num && (this.num = t + 1);
	}
}, be = class {
	deleted = !1;
	records = new de(!0);
	rawIndexes = /* @__PURE__ */ new Map();
	constructor(e, t, n, r) {
		this.rawDatabase = e, this.keyGenerator = r === !0 ? new ye() : null, this.deleted = !1, this.name = t, this.keyPath = n, this.autoIncrement = r;
	}
	getKey(e) {
		let t = this.records.get(e);
		return t === void 0 ? void 0 : structuredClone(t.key);
	}
	getAllKeys(e, t, n) {
		(t === void 0 || t === 0) && (t = Infinity);
		let r = [];
		for (let i of this.records.values(e, n)) if (r.push(structuredClone(i.key)), r.length >= t) break;
		return r;
	}
	getValue(e) {
		let t = this.records.get(e);
		return t === void 0 ? void 0 : structuredClone(t.value);
	}
	getAllValues(e, t, n) {
		(t === void 0 || t === 0) && (t = Infinity);
		let r = [];
		for (let i of this.records.values(e, n)) if (r.push(structuredClone(i.value)), r.length >= t) break;
		return r;
	}
	getAllRecords(e, t, n) {
		(t === void 0 || t === 0) && (t = Infinity);
		let r = [];
		for (let i of this.records.values(e, n)) if (r.push(new M(structuredClone(i.key), structuredClone(i.key), structuredClone(i.value))), r.length >= t) break;
		return r;
	}
	storeRecord(e, t, n) {
		if (this.keyPath !== null) {
			let t = v(this.keyPath, e.value).key;
			t !== void 0 && (e.key = t);
		}
		let r = [];
		if (this.keyGenerator !== null && e.key === void 0) {
			let t = !1, a = this.keyGenerator.num, o = () => {
				t || (t = !0, this.keyGenerator && (this.keyGenerator.num = a));
			};
			if (r.push(o), n && n.push(o), e.key = this.keyGenerator.next(), this.keyPath !== null) {
				if (Array.isArray(this.keyPath)) throw Error("Cannot have an array key path in an object store with a key generator");
				let t = this.keyPath, n = e.value, r, a = 0;
				for (; a >= 0;) {
					if (typeof n != "object") throw new i();
					a = t.indexOf("."), a >= 0 && (r = t.slice(0, a), t = t.slice(a + 1), Object.hasOwn(n, r) || Object.defineProperty(n, r, {
						configurable: !0,
						enumerable: !0,
						writable: !0,
						value: {}
					}), n = n[r]);
				}
				r = t, Object.defineProperty(n, r, {
					configurable: !0,
					enumerable: !0,
					writable: !0,
					value: e.key
				});
			}
		} else this.keyGenerator !== null && typeof e.key == "number" && this.keyGenerator.setIfLarger(e.key);
		let a = this.records.put(e, t), o = !1, s = () => {
			o || (o = !0, a ? this.storeRecord(a, !1) : this.deleteRecord(e.key));
		};
		if (r.push(s), n && n.push(s), a) for (let t of this.rawIndexes.values()) t.records.deleteByValue(e.key);
		try {
			for (let t of this.rawIndexes.values()) t.initialized && t.storeRecord(e);
		} catch (e) {
			if (e.name === "ConstraintError") for (let e of r) e();
			throw e;
		}
		return e.key;
	}
	deleteRecord(e, t) {
		let n = this.records.delete(e);
		if (t) for (let e of n) t.push(() => {
			this.storeRecord(e, !0);
		});
		for (let t of this.rawIndexes.values()) t.records.deleteByValue(e);
	}
	clear(e) {
		let t = this.records.clear();
		if (e) for (let n of t) e.push(() => {
			this.storeRecord(n, !0);
		});
		for (let e of this.rawIndexes.values()) e.records.clear();
	}
	count(e) {
		if (e === void 0 || e.lower === void 0 && e.upper === void 0) return this.records.size();
		let t = 0;
		for (let n of this.records.values(e)) t += 1;
		return t;
	}
}, xe = (e, t = !1) => {
	if (e._closePending = !0, e._rawDatabase.transactions.every((e) => e._state === "finished")) {
		if (e._closed = !0, e._rawDatabase.connections = e._rawDatabase.connections.filter((t) => e !== t), t) {
			let t = new I("close", {
				bubbles: !1,
				cancelable: !1
			});
			t.eventPath = [], e.dispatchEvent(t);
		}
	} else L(() => {
		xe(e, t);
	});
}, Se = (e) => {
	let t;
	if (e._runningVersionchangeTransaction && (t = e._rawDatabase.transactions.findLast((e) => e.mode === "versionchange")), !t) throw new o();
	if (t._state !== "active") throw new u();
	return t;
}, Ce = class extends w {
	_closePending = !1;
	_closed = !1;
	_runningVersionchangeTransaction = !1;
	constructor(e) {
		super(), this._rawDatabase = e, this._rawDatabase.connections.push(this), this.name = e.name, this.version = e.version, this.objectStoreNames = new E(...Array.from(e.rawObjectStores.keys()).sort());
	}
	createObjectStore(e, t = {}) {
		if (e === void 0) throw TypeError();
		let n = Se(this), i = t !== null && t.keyPath !== void 0 ? t.keyPath : null, o = t !== null && t.autoIncrement !== void 0 && t.autoIncrement;
		if (i !== null && N(i), this._rawDatabase.rawObjectStores.has(e)) throw new r();
		if (o && (i === "" || Array.isArray(i))) throw new a();
		let s = [...this.objectStoreNames], c = [...n.objectStoreNames], l = new be(this._rawDatabase, e, i, o);
		return this.objectStoreNames._push(e), this.objectStoreNames._sort(), n._scope.add(e), n._createdObjectStores.add(l), this._rawDatabase.rawObjectStores.set(e, l), n.objectStoreNames = new E(...this.objectStoreNames), n._rollbackLog.push(() => {
			l.deleted = !0, this.objectStoreNames = new E(...s), n.objectStoreNames = new E(...c), n._scope.delete(l.name), this._rawDatabase.rawObjectStores.delete(l.name);
		}), n.objectStore(e);
	}
	deleteObjectStore(e) {
		if (e === void 0) throw TypeError();
		let t = Se(this), n = this._rawDatabase.rawObjectStores.get(e);
		if (n === void 0) throw new s();
		this.objectStoreNames = new E(...Array.from(this.objectStoreNames).filter((t) => t !== e)), t.objectStoreNames = new E(...this.objectStoreNames);
		let r = t._objectStoresCache.get(e), i;
		r && (i = [...r.indexNames], r.indexNames = new E()), t._rollbackLog.push(() => {
			n.deleted = !1, this._rawDatabase.rawObjectStores.set(n.name, n), this.objectStoreNames._push(n.name), t.objectStoreNames._push(n.name), this.objectStoreNames._sort(), r && i && (r.indexNames = new E(...i));
		}), n.deleted = !0, this._rawDatabase.rawObjectStores.delete(e), t._objectStoresCache.delete(e);
	}
	transaction(e, t, n) {
		if (t = t === void 0 ? "readonly" : t, t !== "readonly" && t !== "readwrite" && t !== "versionchange") throw TypeError("Invalid mode: " + t);
		if (this._rawDatabase.transactions.some((e) => e._state === "active" && e.mode === "versionchange" && e.db === this) || this._closePending) throw new o();
		if (Array.isArray(e) || (e = [e]), e.length === 0 && t !== "versionchange") throw new a();
		for (let t of e) if (!this.objectStoreNames.contains(t)) throw new s("No objectStore named " + t + " in this database");
		let r = n?.durability ?? "default";
		if (r !== "default" && r !== "strict" && r !== "relaxed") throw TypeError(`'${r}' (value of 'durability' member of IDBTransactionOptions) is not a valid value for enumeration IDBTransactionDurability`);
		let i = new _e(e, t, r, this);
		return this._rawDatabase.transactions.push(i), this._rawDatabase.processTransactions(), i;
	}
	close() {
		xe(this);
	}
	get [Symbol.toStringTag]() {
		return "IDBDatabase";
	}
}, R = class extends T {
	onupgradeneeded = null;
	onblocked = null;
	get [Symbol.toStringTag]() {
		return "IDBOpenDBRequest";
	}
}, z = class extends I {
	constructor(e, t = {}) {
		super(e), this.newVersion = t.newVersion === void 0 ? null : t.newVersion, this.oldVersion = t.oldVersion === void 0 ? 0 : t.oldVersion;
	}
	get [Symbol.toStringTag]() {
		return "IDBVersionChangeEvent";
	}
};
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/intersection.js
function we(e, t) {
	return "intersection" in e ? e.intersection(t) : new Set([...e].filter((e) => t.has(e)));
}
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/Database.js
var Te = class {
	transactions = [];
	rawObjectStores = /* @__PURE__ */ new Map();
	connections = [];
	constructor(e, t) {
		this.name = e, this.version = t, this.processTransactions = this.processTransactions.bind(this);
	}
	processTransactions() {
		L(() => {
			let e = this.transactions.filter((e) => e._started && e._state !== "finished"), t = this.transactions.filter((e) => !e._started && e._state !== "finished"), n = t.find((n, r) => !e.some((e) => !(n.mode === "readonly" && e.mode === "readonly") && we(e._scope, n._scope).size > 0) && !t.slice(0, r).some((e) => we(e._scope, n._scope).size > 0));
			n && (n.addEventListener("complete", this.processTransactions), n.addEventListener("abort", this.processTransactions), n._start());
		});
	}
};
//#endregion
//#region node_modules/fake-indexeddb/build/esm/lib/validateRequiredArguments.js
function B(e, t, n) {
	if (e < t) throw TypeError(`${n}: At least ${t} ${t === 1 ? "argument" : "arguments"} required, but only ${arguments.length} passed`);
}
//#endregion
//#region node_modules/fake-indexeddb/build/esm/FDBFactory.js
var Ee = (e, t, n) => {
	let r = e.get(t) ?? Promise.resolve();
	e.set(t, r.then(n));
}, De = (e, t, n, r) => {
	if (n.some((e) => !e._closed && !e._closePending)) {
		L(() => De(e, t, n, r));
		return;
	}
	e.delete(t), r(null);
}, Oe = (e, t, n, r, i) => {
	Ee(t, n, () => new Promise((t) => {
		let a = e.get(n), o = a === void 0 ? 0 : a.version, s = (e) => {
			try {
				e ? i(e) : i(null, o);
			} finally {
				t();
			}
		};
		try {
			let t = e.get(n);
			if (t === void 0) {
				s(null);
				return;
			}
			let i = t.connections.filter((e) => !e._closed);
			for (let e of i) e._closePending || L(() => {
				let n = new z("versionchange", {
					newVersion: null,
					oldVersion: t.version
				});
				e.dispatchEvent(n);
			});
			L(() => {
				i.some((e) => !e._closed && !e._closePending) && L(() => {
					let e = new z("blocked", {
						newVersion: null,
						oldVersion: t.version
					});
					r.dispatchEvent(e);
				}), De(e, n, i, s);
			});
		} catch (e) {
			s(e);
		}
	}));
}, ke = (e, t, r, i) => {
	e._runningVersionchangeTransaction = !0;
	let a = e._oldVersion = e.version, o = e._rawDatabase.connections.filter((t) => e !== t);
	for (let e of o) !e._closed && !e._closePending && L(() => {
		let n = new z("versionchange", {
			newVersion: t,
			oldVersion: a
		});
		e.dispatchEvent(n);
	});
	L(() => {
		o.some((e) => !e._closed && !e._closePending) && L(() => {
			let e = new z("blocked", {
				newVersion: t,
				oldVersion: a
			});
			r.dispatchEvent(e);
		});
		let s = () => {
			if (o.some((e) => !e._closed && !e._closePending)) {
				L(s);
				return;
			}
			e._rawDatabase.version = t, e.version = t;
			let c = e.transaction(Array.from(e.objectStoreNames), "versionchange");
			c._openRequest = r, r.result = e, r.readyState = "done", r.transaction = c, c._rollbackLog.push(() => {
				e._rawDatabase.version = a, e.version = a;
			}), c._state = "active";
			let l = new z("upgradeneeded", {
				newVersion: t,
				oldVersion: a
			}), u = !1;
			try {
				r.dispatchEvent(l);
			} catch {
				u = !0;
			}
			let d = () => {
				c._state === "active" && (c._state = "inactive", u && c._abort("AbortError"));
			};
			u ? d() : L(d), c.addEventListener("error", () => {
				e._runningVersionchangeTransaction = !1, e._oldVersion = void 0;
			}), c.addEventListener("abort", () => {
				e._runningVersionchangeTransaction = !1, e._oldVersion = void 0, r.transaction = null, L(() => {
					i(new n());
				});
			}), c.addEventListener("complete", () => {
				e._runningVersionchangeTransaction = !1, e._oldVersion = void 0, r.transaction = null, L(() => {
					e._closePending ? i(new n()) : i(null);
				});
			});
		};
		s();
	});
}, Ae = (e, t, n, r, i, a) => {
	Ee(t, n, () => new Promise((t) => {
		let o = (e) => {
			try {
				e ? a(e) : a(null, c);
			} finally {
				t();
			}
		}, s = e.get(n);
		if (s === void 0 && (s = new Te(n, 0), e.set(n, s)), r === void 0 && (r = s.version === 0 ? 1 : s.version), s.version > r) return o(new d());
		let c = new Ce(s);
		s.version < r ? ke(c, r, i, (e) => {
			o(e);
		}) : o(null);
	}));
}, je = class {
	_databases = /* @__PURE__ */ new Map();
	_connectionQueues = /* @__PURE__ */ new Map();
	cmp(e, t) {
		return B(arguments.length, 2, "IDBFactory.cmp"), g(e, t);
	}
	deleteDatabase(e) {
		B(arguments.length, 1, "IDBFactory.deleteDatabase");
		let t = new R();
		return t.source = null, L(() => {
			Oe(this._databases, this._connectionQueues, e, t, (e, n) => {
				if (e) {
					t.error = new DOMException(e.message, e.name), t.readyState = "done";
					let n = new I("error", {
						bubbles: !0,
						cancelable: !0
					});
					n.eventPath = [], t.dispatchEvent(n);
					return;
				}
				t.result = void 0, t.readyState = "done";
				let r = new z("success", {
					newVersion: null,
					oldVersion: n
				});
				t.dispatchEvent(r);
			});
		}), t;
	}
	open(e, t) {
		if (B(arguments.length, 1, "IDBFactory.open"), arguments.length > 1 && t !== void 0 && (t = O(t, "MAX_SAFE_INTEGER")), t === 0) throw TypeError("Database version cannot be 0");
		let n = new R();
		return n.source = null, L(() => {
			Ae(this._databases, this._connectionQueues, e, t, n, (e, t) => {
				if (e) {
					n.result = void 0, n.readyState = "done", n.error = new DOMException(e.message, e.name);
					let t = new I("error", {
						bubbles: !0,
						cancelable: !0
					});
					t.eventPath = [], n.dispatchEvent(t);
					return;
				}
				n.result = t, n.readyState = "done";
				let r = new I("success");
				r.eventPath = [], n.dispatchEvent(r);
			});
		}), n;
	}
	databases() {
		return Promise.resolve(Array.from(this._databases.entries(), ([e, t]) => {
			let n = t.connections.find((e) => e._runningVersionchangeTransaction);
			return {
				name: e,
				version: n ? n._oldVersion : t.version
			};
		}).filter(({ version: e }) => e > 0));
	}
	get [Symbol.toStringTag]() {
		return "IDBFactory";
	}
}, Me = new je(), Ne = typeof window < "u" ? window : typeof WorkerGlobalScope < "u" ? self : typeof global < "u" ? global : Function("return this;")(), V = (e) => ({
	value: e,
	enumerable: !1,
	configurable: !0,
	writable: !0
});
Object.defineProperties(Ne, {
	indexedDB: V(Me),
	IDBCursor: V(x),
	IDBCursorWithValue: V(S),
	IDBDatabase: V(Ce),
	IDBFactory: V(je),
	IDBIndex: V(j),
	IDBKeyRange: V(_),
	IDBObjectStore: V(F),
	IDBOpenDBRequest: V(R),
	IDBRecord: V(M),
	IDBRequest: V(T),
	IDBTransaction: V(_e),
	IDBVersionChangeEvent: V(z)
});
//#endregion
//#region hta.js
var Pe = new Uint8Array([
	72,
	84,
	65,
	49
]), H = {
	nil: 0,
	false: 1,
	true: 2,
	i64: 3,
	string: 4,
	bytes: 5,
	keyword: 6,
	symbol: 7,
	list: 8,
	vector: 9,
	set: 10,
	map: 11,
	handle: 12,
	f64: 15
}, U = new TextEncoder(), W = new TextDecoder("utf-8", { fatal: !0 }), G = class {
	constructor(e) {
		this.name = e;
	}
}, Fe = class {
	constructor(e) {
		this.name = e;
	}
}, Ie = class {
	constructor(e, t, n, r = null, i = "ht", a = "handle") {
		this.owner = e, this.type = t, this.id = BigInt(n), this.context = r, this.displayTag = i, this.displayKind = a, this.released = !1;
	}
	release() {
		this.released || (this.released = !0, this.context && this.context.releaseHandle(this));
	}
	toString() {
		return `#${this.displayTag}[:${this.displayKind} ${this.id}]`;
	}
};
function Le(e) {
	let t = [...Pe];
	return K(t, e), Uint8Array.from(t);
}
function Re(e) {
	let t = e instanceof Uint8Array ? e : new Uint8Array(e);
	if (t.length < 4 || !Pe.every((e, n) => t[n] === e)) throw Error("hta/value-malformed: invalid HTA1 header");
	let n = new We(t, 4), r = n.value();
	if (n.cursor !== t.length) throw Error("hta/value-malformed: trailing bytes");
	return r;
}
function K(e, t) {
	if (t == null) e.push(H.nil);
	else if (t === !1) e.push(H.false);
	else if (t === !0) e.push(H.true);
	else if (typeof t == "bigint" || Number.isSafeInteger(t) && !Object.is(t, -0)) e.push(H.i64), He(e, BigInt(t));
	else if (typeof t == "number") e.push(H.f64), Ue(e, t);
	else if (typeof t == "string") e.push(H.string), J(e, U.encode(t));
	else if (t instanceof Uint8Array) e.push(H.bytes), J(e, t);
	else if (t instanceof G) e.push(H.keyword), J(e, U.encode(t.name));
	else if (t instanceof Fe) e.push(H.symbol), J(e, U.encode(t.name));
	else if (t instanceof Ie) {
		if (t.released) throw Error("hta/handle-released");
		e.push(H.handle), J(e, U.encode(t.owner)), J(e, U.encode(t.type)), He(e, t.id);
	} else if (Array.isArray(t)) e.push(H.vector), ze(e, t);
	else if (t instanceof Set) e.push(H.set), Be(e, [...t]);
	else if (t instanceof Map) {
		let n = [...t].map(([e, t]) => [q(e), q(t)]).sort((e, t) => Ve(e[0], t[0]));
		e.push(H.map), Y(e, n.length);
		for (let [t, r] of n) e.push(...t, ...r);
	} else throw Error(`hta/value-unsupported: ${Object.prototype.toString.call(t)}`);
}
function q(e) {
	let t = [];
	return K(t, e), t;
}
function ze(e, t) {
	Y(e, t.length);
	for (let n of t) K(e, n);
}
function Be(e, t) {
	let n = t.map(q).sort(Ve);
	Y(e, n.length);
	for (let t of n) e.push(...t);
}
function Ve(e, t) {
	for (let n = 0; n < Math.min(e.length, t.length); n++) if (e[n] !== t[n]) return e[n] - t[n];
	return e.length - t.length;
}
function J(e, t) {
	Y(e, t.length), e.push(...t);
}
function Y(e, t) {
	if (t < 0 || t > 4294967295) throw Error("hta/value-too-large");
	e.push(t >>> 24, t >>> 16 & 255, t >>> 8 & 255, t & 255);
}
function He(e, t) {
	let n = BigInt.asUintN(64, t);
	for (let t = 56n; t >= 0n; t -= 8n) e.push(Number(n >> t & 255n));
}
function Ue(e, t) {
	let n = /* @__PURE__ */ new Uint8Array(8);
	new DataView(n.buffer).setFloat64(0, t, !1), e.push(...n);
}
var We = class {
	constructor(e, t) {
		this.bytes = e, this.cursor = t;
	}
	take(e) {
		let t = this.cursor + e;
		if (t > this.bytes.length) throw Error("hta/value-malformed: truncated value");
		let n = this.bytes.subarray(this.cursor, t);
		return this.cursor = t, n;
	}
	u32() {
		let e = this.take(4);
		return e[0] * 16777216 + (e[1] << 16) + (e[2] << 8) + e[3] >>> 0;
	}
	data() {
		return this.take(this.u32());
	}
	sequence() {
		let e = this.u32(), t = [];
		for (let n = 0; n < e; n++) t.push(this.value());
		return t;
	}
	value() {
		let e = this.take(1)[0];
		if (e === H.nil) return null;
		if (e === H.false) return !1;
		if (e === H.true) return !0;
		if (e === H.i64) {
			let e = this.take(8), t = 0n;
			for (let n of e) t = t << 8n | BigInt(n);
			return t = BigInt.asIntN(64, t), t >= BigInt(-(2 ** 53 - 1)) && t <= BigInt(2 ** 53 - 1) ? Number(t) : t;
		}
		if (e === H.f64) {
			let e = this.take(8);
			return new DataView(e.buffer, e.byteOffset, 8).getFloat64(0, !1);
		}
		if (e === H.string) return W.decode(this.data());
		if (e === H.bytes) return this.data().slice();
		if (e === H.keyword) return new G(W.decode(this.data()));
		if (e === H.symbol) return new Fe(W.decode(this.data()));
		if (e === H.list || e === H.vector) return this.sequence();
		if (e === H.set) return new Set(this.sequence());
		if (e === H.map) {
			let e = this.u32(), t = /* @__PURE__ */ new Map();
			for (let n = 0; n < e; n++) t.set(this.value(), this.value());
			return t;
		}
		if (e === H.handle) {
			let e = W.decode(this.data()), t = W.decode(this.data()), n = this.take(8), r = 0n;
			for (let e of n) r = r << 8n | BigInt(e);
			return new Ie(e, t, r);
		}
		throw Error("hta/value-malformed: unknown value tag");
	}
}, Ge;
function Ke(e) {
	return Ge ??= import(e).then((e) => ({
		module: e,
		loader: new e.NoirBrowserLoader({ cache: new e.MemoryArtifactCache() })
	})), Ge;
}
async function qe(e, t, n) {
	let { module: r, loader: i } = await Ke(e), a = n.map(Z);
	if (t === "compile") {
		let e = a[0] ?? {}, t = {
			name: X(e, "name"),
			source: X(e, "source"),
			noirVersion: e.noirVersion ?? e["noir-version"] ?? r.NOIR_VERSION,
			backendVersion: e.backendVersion ?? e["backend-version"] ?? r.BACKEND_ID
		};
		return Je(await i.compile(t));
	}
	if (t === "prove") {
		let e = Xe(a[0]);
		return Ye(await i.prove(e, a[1]));
	}
	if (t === "verify") return i.verify(Xe(a[0]), Ze(a[1]));
	throw Error(`noir/operation-unknown: ${t}`);
}
function Je(e) {
	return {
		format: e.format,
		programKey: e.programKey,
		loaderId: e.loaderId,
		compilerVersion: e.compilerVersion,
		backendVersion: e.backendVersion,
		circuitJson: JSON.stringify(e.circuit)
	};
}
function Ye(e) {
	return {
		format: e.format,
		programKey: e.programKey,
		loaderId: e.loaderId,
		proof: e.proof,
		publicInputs: [...e.publicInputs]
	};
}
function Xe(e) {
	if (!e || e.format !== "hara/ledger.noir/v1") throw TypeError("noir/artifact-format: expected hara/ledger.noir/v1");
	return {
		format: e.format,
		programKey: e.programKey,
		loaderId: e.loaderId,
		compilerVersion: e.compilerVersion,
		backendVersion: e.backendVersion,
		circuit: JSON.parse(X(e, "circuitJson"))
	};
}
function Ze(e) {
	if (!e || e.format !== "hara.noir.proof/v1") throw TypeError("noir/proof-format: expected hara.noir.proof/v1");
	return {
		format: e.format,
		programKey: e.programKey,
		loaderId: e.loaderId,
		proof: e.proof,
		publicInputs: e.publicInputs
	};
}
function X(e, t) {
	let n = e?.[t];
	if (typeof n != "string" || n.length === 0) throw TypeError(`noir/${t} must be a non-empty string`);
	return n;
}
function Z(e) {
	if (e instanceof Map) {
		let t = {};
		for (let [n, r] of e) {
			let e = typeof n == "string" ? n : n?.name;
			if (typeof e != "string") throw TypeError("noir/map keys must be strings or keywords");
			t[Qe(e)] = Z(r);
		}
		return t;
	}
	return Array.isArray(e) ? e.map(Z) : e instanceof Set ? [...e].map(Z) : e;
}
function Qe(e) {
	return e.replace(/-([a-z])/g, (e, t) => t.toUpperCase());
}
//#endregion
//#region ../extensions/ledger-noir/node/worker.mjs
var $e = new URL("../assets/noir-loader.js", "" + import.meta.url).toString(), et = /* @__PURE__ */ new Set(), Q = /* @__PURE__ */ new Uint8Array(), $ = null;
console.log = (...e) => console.error(...e), console.info = (...e) => console.error(...e), process.stdin.on("data", (e) => {
	let t = new Uint8Array(Q.length + e.length);
	t.set(Q), t.set(e, Q.length), Q = t, tt();
}), process.stdin.on("end", () => process.exit(0));
function tt() {
	for (;;) {
		if ($ === null) {
			if (Q.length < 4) return;
			if ($ = new DataView(Q.buffer, Q.byteOffset, 4).getUint32(0, !1), Q = Q.slice(4), $ === 0 || $ > 64 * 1024 * 1024) throw Error("hta/process-frame-size");
		}
		if (Q.length < $) return;
		let e = Q.slice(0, $);
		Q = Q.slice($), $ = null, nt(Re(e));
	}
}
async function nt(e) {
	let [t, n, r, i] = e;
	if (t === "handshake") {
		rt(["ready", 1]);
		return;
	}
	if (t === "shutdown") {
		process.exit(0);
		return;
	}
	if (t === "cancel") {
		et.add(Number(n));
		return;
	}
	if (t !== "call") throw Error(`hta/process-event-unknown: ${t}`);
	let a = Number(n);
	try {
		let e = await qe($e, r, i);
		et.delete(a) || rt([
			"result",
			a,
			it(e)
		]);
	} catch (e) {
		et.delete(a) || rt([
			"error",
			a,
			at(e)
		]);
	}
}
function rt(e) {
	let t = Le(e), n = /* @__PURE__ */ new Uint8Array(4);
	new DataView(n.buffer).setUint32(0, t.length, !1), process.stdout.write(n), process.stdout.write(t);
}
function it(e) {
	if (typeof e != "object" || !e) return e ?? null;
	if (Array.isArray(e)) return e.map(it);
	let t = /* @__PURE__ */ new Map();
	for (let [n, r] of Object.entries(e)) t.set(new G(n), it(r));
	return t;
}
function at(e) {
	let t = String(e?.message ?? e), n = t.indexOf(":"), r = n > 0 ? t.slice(0, n) : "noir/error";
	return /* @__PURE__ */ new Map([
		[new G("code"), new G(r)],
		[new G("message"), t],
		[new G("origin"), new G("node")],
		[new G("retryable"), !1]
	]);
}
//#endregion
