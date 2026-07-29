const MAGIC = new Uint8Array([0x48, 0x54, 0x41, 0x31]);
const TAG = { nil: 0, false: 1, true: 2, i64: 3, string: 4, bytes: 5, keyword: 6, symbol: 7, list: 8, vector: 9, set: 10, map: 11, handle: 12, namespace: 13, var: 14, f64: 15, atom: 16, array: 17, object: 18 };
const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });

export class HtaKeyword { constructor(name) { this.name = name; } }
export class HtaSymbol { constructor(name) { this.name = name; } }
export class HtaVar { constructor(symbol,value=null){this.symbol=symbol;this.value=value;} toString(){return `#'${this.symbol.name}`;} }
export class HtaAtom { constructor(value){this.value=value;} toString(){return `#atom <${displayHta(this.value)}>`;} }
export class HtaArray { constructor(values){this.values=values;} toString(){return `(array${this.values.length?` ${this.values.map(displayHta).join(" ")}`:""})`;} }
export class HtaObject { constructor(entries){this.entries=entries;} toString(){return `(object${this.entries.length?` ${this.entries.map(([key,value])=>`${JSON.stringify(key)} ${displayHta(value)}`).join(" ")}`:""})`;} }
function displayHta(value){if(value===null)return"nil";if(typeof value==="string")return JSON.stringify(value);if(value instanceof HtaKeyword)return`:${value.name}`;if(value instanceof Map)return`{${[...value].map(([key,item])=>`${displayHta(key)} ${displayHta(item)}`).join(" ")}}`;if(Array.isArray(value))return`[${value.map(displayHta).join(" ")}]`;return String(value);}
export class HtaHandle { constructor(owner,type,id,context=null,displayTag="ht",displayKind="handle"){this.owner=owner;this.type=type;this.id=BigInt(id);this.context=context;this.displayTag=displayTag;this.displayKind=displayKind;this.released=false;} release(){if(this.released)return;this.released=true;if(this.context)this.context.releaseHandle(this);} toString(){return `#${this.displayTag}[:${this.displayKind} ${this.id}]`;} }

/** Browser host adapter for the portable Hara promise-provider contract. */
export class BrowserPromiseProvider {
  constructor(options={}) {
    this.enqueue=options.enqueue ?? (task=>queueMicrotask(task));
    this.schedule=options.schedule ?? ((task,milliseconds)=>setTimeout(task,milliseconds));
    this.cancelSchedule=options.cancelSchedule ?? (timer=>clearTimeout(timer));
  }
  create(executor) {
    let settled=false,rejectPromise=()=>{},cancelAction=()=>{};
    const promise=new Promise((resolve,reject)=>{
      rejectPromise=reject;
      const settle=(callback)=>(value)=>{if(settled)return false;settled=true;callback(value);return true;};
      const onCancel=(action)=>{cancelAction=typeof action==="function"?action:()=>{};};
      try{executor(settle(resolve),settle(reject),onCancel);}catch(error){settle(reject)(error);}
    });
    promise.cancel=()=>{if(settled)return false;cancelAction();settled=true;rejectPromise(new Error("cancelled"));return true;};
    return promise;
  }
  run(task) {
    return this.create((resolve,reject,onCancel)=>{
      let cancelled=false;onCancel(()=>{cancelled=true;});
      this.enqueue(()=>{if(cancelled)return;try{resolve(task());}catch(error){reject(error);}});
    });
  }
  delay(milliseconds,task) {
    return this.create((resolve,reject,onCancel)=>{
      const timer=this.schedule(()=>{try{resolve(task());}catch(error){reject(error);}},milliseconds);
      onCancel(()=>this.cancelSchedule(timer));
    });
  }
  all(values) { return this.create((resolve,reject)=>Promise.all(values).then(resolve,reject)); }
  then(source,callback) { return this.create((resolve,reject)=>Promise.resolve(source).then(callback).then(resolve,reject)); }
  catch(source,callback) { return this.create((resolve,reject)=>Promise.resolve(source).catch(callback).then(resolve,reject)); }
  finally(source,callback) { return this.create((resolve,reject)=>Promise.resolve(source).finally(callback).then(resolve,reject)); }
}

export function encodeHta(value) {
  const output = [...MAGIC];
  writeValue(output, value);
  return Uint8Array.from(output);
}

export function decodeHta(input) {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
  if (bytes.length < 4 || !MAGIC.every((byte, index) => bytes[index] === byte)) {
    throw new Error("hta/value-malformed: invalid HTA1 header");
  }
  const reader = new Reader(bytes, 4);
  const value = reader.value();
  if (reader.cursor !== bytes.length) throw new Error("hta/value-malformed: trailing bytes");
  return value;
}

export function parseHtaManifest(source) {
  const reader = new ManifestReader(source);
  const value = reader.value();
  reader.space();
  if (reader.cursor !== source.length || !(value instanceof Map)) throw new Error("hta/manifest-malformed: expected one EDN map");
  const namespace = manifestField(value,"namespace"), identity = manifestField(value,"identity"), providerValue = manifestField(value,"provider"), module = manifestField(value,"module"), abiValue = manifestField(value,"abi");
  if (typeof namespace !== "string" || !/^[a-z][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*)+$/.test(namespace)) throw new Error("hta/manifest-malformed: invalid namespace");
  if (identity !== undefined && (typeof identity !== "string" || !/^[a-z][a-z0-9-]*\/[a-z][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*)*$/.test(identity))) throw new Error("hta/manifest-malformed: invalid identity");
  if (!(providerValue instanceof HtaKeyword) || !["wasm","hta"].includes(providerValue.name)) throw new Error("hta/manifest-malformed: provider must be :wasm or :hta");
  if (!(abiValue instanceof HtaKeyword)) throw new Error("hta/manifest-malformed: abi must be a keyword");
  const provider=providerValue.name,abi=abiValue.name;
  let browserTarget;
  if (provider === "wasm") {
    if (!validPackagePath(module,".wasm")) throw new Error("hta/manifest-malformed: invalid module");
  } else {
    if (module !== undefined || abi !== "hta.v1") throw new Error("hta/manifest-malformed: HTA targets require :abi :hta.v1 without :module");
    const targets=manifestField(value,"targets"),browser=targets instanceof Map?manifestField(targets,"browser"):undefined;
    const browserModule=browser instanceof Map?manifestField(browser,"module"):undefined;
    const runtime=browser instanceof Map?manifestField(browser,"runtime"):undefined;
    if (!validPackagePath(browserModule,".mjs") || !(runtime instanceof HtaKeyword) || runtime.name !== "web-worker") throw new Error("hta/manifest-malformed: missing :browser web-worker target");
    browserTarget=Object.freeze({module:browserModule,runtime:runtime.name});
  }
  const assetsValue=manifestField(value,"assets"),assets=[];
  if (assetsValue !== undefined) {
    if (!Array.isArray(assetsValue) || assetsValue.some(asset=>!validPackagePath(asset))) throw new Error("hta/manifest-malformed: invalid assets");
    assets.push(...assetsValue);
  }
  const handleTags = {}, handles = manifestField(value,"handles");
  if (handles !== undefined) {
    if (!(handles instanceof Map)) throw new Error("hta/manifest-malformed: handles must be a map");
    for (const [type,spec] of handles) {
      const tag = spec instanceof Map ? manifestField(spec,"tag") : undefined;
      if (typeof type !== "string" || !/^[a-z][a-z0-9-]*$/.test(type) || !(tag instanceof HtaSymbol) || !/^[a-z][a-z0-9-]*(\.[a-z][a-z0-9-]*)*$/.test(tag.name)) throw new Error("hta/manifest-malformed: invalid handle tag");
      handleTags[type] = tag.name;
    }
  }
  return Object.freeze({namespace,identity,provider,module,abi,browserTarget,assets:Object.freeze(assets),handleTags:Object.freeze(handleTags)});
}

function validPackagePath(value,suffix) {
  return typeof value === "string" && value.length>0 && !value.startsWith("/") && !value.split("/").includes("..") && !value.includes(":") && (!suffix || value.endsWith(suffix));
}

export async function loadHtaExtension({worker,descriptor,descriptorUrl,packageUrl,moduleBytes,hostCalls={}}) {
  if (descriptor === undefined) {
    if (!descriptorUrl) throw new Error("hta/manifest-missing: descriptor or descriptorUrl is required");
    const response = await fetch(descriptorUrl);
    if (!response.ok) throw new Error(`hta/manifest-load-failed: ${response.status}`);
    descriptor = await response.text();
  }
  const manifest = parseHtaManifest(descriptor);
  const base = packageUrl ?? descriptorUrl;
  let moduleUrl;
  if (manifest.provider === "wasm" && moduleBytes === undefined) {
    if (!base) throw new Error("hta/manifest-missing: packageUrl is required with inline descriptors");
    moduleUrl = new URL(manifest.module,base).toString();
  }
  if (manifest.provider === "hta" && !worker) {
    if (!base) throw new Error("hta/manifest-missing: packageUrl is required with inline descriptors");
    worker = new Worker(new URL(manifest.browserTarget.module,base),{type:"module",name:`hara-${manifest.namespace}`});
  }
  if (!worker) throw new Error("hta/worker-missing: worker is required for WASM providers");
  const context = new HtaContext({worker,moduleUrl,moduleBytes,hostCalls,handleTags:manifest.handleTags});
  context.manifest = manifest;
  return context;
}

function manifestField(map,name) { for (const [key,value] of map) if (key instanceof HtaKeyword && key.name===name) return value; }

class ManifestReader {
  constructor(source){this.source=source;this.cursor=0;}
  space(){while(this.cursor<this.source.length){const ch=this.source[this.cursor];if(/[\s,]/.test(ch)){this.cursor++;continue;}if(ch===';'){while(this.cursor<this.source.length&&this.source[this.cursor]!=='\n')this.cursor++;continue;}break;}}
  value(){this.space();const ch=this.source[this.cursor++];if(ch===undefined)throw new Error("hta/manifest-malformed: unexpected EOF");if(ch==='{')return this.map();if(ch==='[')return this.vector();if(ch==='\"')return this.string();if(ch===':')return new HtaKeyword(this.token());this.cursor--;const token=this.token();if(token==='nil')return null;if(token==='true')return true;if(token==='false')return false;if(/^-?[0-9]+$/.test(token))return Number(token);return new HtaSymbol(token);}
  map(){const result=new Map();for(;;){this.space();if(this.source[this.cursor]==='}'){this.cursor++;return result;}const key=this.value();this.space();if(this.source[this.cursor]==='}')throw new Error("hta/manifest-malformed: map value missing");result.set(key,this.value());}}
  vector(){const result=[];for(;;){this.space();if(this.source[this.cursor]===']'){this.cursor++;return result;}result.push(this.value());}}
  string(){let result='';while(this.cursor<this.source.length){const ch=this.source[this.cursor++];if(ch==='\"')return result;if(ch==='\\'){const escaped=this.source[this.cursor++];if(escaped==='u'){const code=this.source.slice(this.cursor,this.cursor+4);if(!/^[0-9a-fA-F]{4}$/.test(code))throw new Error("hta/manifest-malformed: invalid unicode escape");result+=String.fromCharCode(parseInt(code,16));this.cursor+=4;}else{const escapes={n:'\n',r:'\r',t:'\t',b:'\b',f:'\f','\"':'\"','\\':'\\'};if(!(escaped in escapes))throw new Error("hta/manifest-malformed: invalid string escape");result+=escapes[escaped];}}else result+=ch;}throw new Error("hta/manifest-malformed: unterminated string");}
  token(){this.space();const start=this.cursor;while(this.cursor<this.source.length&&!/[\s,{}\[\]\"]/ .test(this.source[this.cursor]))this.cursor++;if(start===this.cursor)throw new Error("hta/manifest-malformed: invalid token");return this.source.slice(start,this.cursor);}
}

function writeValue(output, value) {
  if (value === null || value === undefined) output.push(TAG.nil);
  else if (value === false) output.push(TAG.false);
  else if (value === true) output.push(TAG.true);
  else if (typeof value === "bigint" || (Number.isSafeInteger(value) && !Object.is(value, -0))) {
    output.push(TAG.i64); writeI64(output, BigInt(value));
  } else if (typeof value === "number") {
    output.push(TAG.f64); writeF64(output, value);
  } else if (typeof value === "string") { output.push(TAG.string); writeBytes(output, encoder.encode(value)); }
  else if (value instanceof Uint8Array) { output.push(TAG.bytes); writeBytes(output, value); }
  else if (value instanceof HtaKeyword) { output.push(TAG.keyword); writeBytes(output, encoder.encode(value.name)); }
  else if (value instanceof HtaSymbol) { output.push(TAG.symbol); writeBytes(output, encoder.encode(value.name)); }
  else if (value instanceof HtaVar) { output.push(TAG.var); writeValue(output,value.symbol); writeValue(output,value.value); }
  else if (value instanceof HtaAtom) { output.push(TAG.atom); writeValue(output,value.value); }
  else if (value instanceof HtaArray) { output.push(TAG.array); writeSequence(output,value.values); }
  else if (value instanceof HtaObject) { output.push(TAG.object); writeU32(output,value.entries.length);for(const [key,item] of value.entries){writeValue(output,key);writeValue(output,item);} }
  else if (value instanceof HtaHandle) { if(value.released)throw new Error("hta/handle-released");output.push(TAG.handle);writeBytes(output,encoder.encode(value.owner));writeBytes(output,encoder.encode(value.type));writeI64(output,value.id); }
  else if (Array.isArray(value)) { output.push(TAG.vector); writeSequence(output, value); }
  else if (value instanceof Set) { output.push(TAG.set); writeCanonical(output, [...value]); }
  else if (value instanceof Map) {
    const entries = [...value].map(([key, item]) => [bare(key), bare(item)]).sort((a, b) => compare(a[0], b[0]));
    output.push(TAG.map); writeU32(output, entries.length);
    for (const [key, item] of entries) output.push(...key, ...item);
  } else throw new Error(`hta/value-unsupported: ${Object.prototype.toString.call(value)}`);
}

function bare(value) { const output = []; writeValue(output, value); return output; }
function writeSequence(output, values) { writeU32(output, values.length); for (const value of values) writeValue(output, value); }
function writeCanonical(output, values) { const encoded = values.map(bare).sort(compare); writeU32(output, encoded.length); for (const value of encoded) output.push(...value); }
function compare(left, right) { for (let i=0;i<Math.min(left.length,right.length);i++) if(left[i]!==right[i]) return left[i]-right[i]; return left.length-right.length; }
function writeBytes(output, bytes) { writeU32(output, bytes.length); output.push(...bytes); }
function writeU32(output, value) { if(value<0||value>0xffff_ffff)throw new Error("hta/value-too-large"); output.push(value>>>24,(value>>>16)&255,(value>>>8)&255,value&255); }
function writeI64(output, value) { const normalized=BigInt.asUintN(64,value); for(let shift=56n;shift>=0n;shift-=8n)output.push(Number((normalized>>shift)&255n)); }
function writeF64(output, value) { const bytes=new Uint8Array(8);new DataView(bytes.buffer).setFloat64(0,value,false);output.push(...bytes); }

class Reader {
  constructor(bytes, cursor) { this.bytes=bytes; this.cursor=cursor; }
  take(size) { const end=this.cursor+size; if(end>this.bytes.length)throw new Error("hta/value-malformed: truncated value"); const value=this.bytes.subarray(this.cursor,end);this.cursor=end;return value; }
  u32() { const value=this.take(4); return ((value[0]*0x1000000)+(value[1]<<16)+(value[2]<<8)+value[3])>>>0; }
  data() { return this.take(this.u32()); }
  sequence() { const size=this.u32(), result=[]; for(let i=0;i<size;i++)result.push(this.value()); return result; }
  value() {
    const tag=this.take(1)[0];
    if(tag===TAG.nil)return null;if(tag===TAG.false)return false;if(tag===TAG.true)return true;
    if(tag===TAG.i64){const bytes=this.take(8);let value=0n;for(const byte of bytes)value=(value<<8n)|BigInt(byte);value=BigInt.asIntN(64,value);return value>=BigInt(Number.MIN_SAFE_INTEGER)&&value<=BigInt(Number.MAX_SAFE_INTEGER)?Number(value):value;}
    if(tag===TAG.f64){const bytes=this.take(8);return new DataView(bytes.buffer,bytes.byteOffset,8).getFloat64(0,false);}
    if(tag===TAG.string)return decoder.decode(this.data());if(tag===TAG.bytes)return this.data().slice();
    if(tag===TAG.keyword)return new HtaKeyword(decoder.decode(this.data()));if(tag===TAG.symbol)return new HtaSymbol(decoder.decode(this.data()));
    if(tag===TAG.list||tag===TAG.vector)return this.sequence();if(tag===TAG.set)return new Set(this.sequence());
    if(tag===TAG.map){const size=this.u32(),result=new Map();for(let i=0;i<size;i++)result.set(this.value(),this.value());return result;}
    if(tag===TAG.var){const symbol=this.value(),value=this.value();if(!(symbol instanceof HtaSymbol))throw new Error("hta/value-malformed: invalid var symbol");return new HtaVar(symbol,value);}
    if(tag===TAG.atom)return new HtaAtom(this.value());
    if(tag===TAG.array)return new HtaArray(this.sequence());
    if(tag===TAG.object){const size=this.u32(),entries=[];for(let i=0;i<size;i++){const key=this.value();if(typeof key!=="string")throw new Error("hta/value-malformed: invalid object key");entries.push([key,this.value()]);}return new HtaObject(entries);}
    if(tag===TAG.handle){const owner=decoder.decode(this.data()),type=decoder.decode(this.data()),bytes=this.take(8);let id=0n;for(const byte of bytes)id=(id<<8n)|BigInt(byte);return new HtaHandle(owner,type,id);}
    throw new Error("hta/value-malformed: unknown value tag");
  }
}

export class HtaContext {
  constructor({ worker, moduleUrl, moduleBytes, hostCalls = {}, handleTags = {}, promiseProvider = new BrowserPromiseProvider(), kernelId = null }) {
    this.worker=worker;this.hostCalls=hostCalls;this.handleTags=handleTags;this.promiseProvider=promiseProvider;this.kernelId=kernelId;this.next=1;this.pending=new Map();this.sessionFilesystems=new Map();this.sessions=new Map();
    this.ready=new Promise((resolve,reject)=>{this.readyResolve=resolve;this.readyReject=reject;});
    worker.addEventListener("message", event=>this.message(event.data));
    worker.addEventListener("error", error=>this.fail(error));
    worker.postMessage({type:"init",moduleUrl,moduleBytes});
  }
  call(target, args=[]) { let id=null,cancelled=false;
    return this.promiseProvider.create((resolve,reject,onCancel)=>{
      onCancel(()=>{cancelled=true;if(id!==null)this.worker.postMessage({type:"cancel",id});});
      this.ready.then(()=>{validateHandles(args,this);id=this.next++;this.pending.set(id,{resolve,reject});this.worker.postMessage({type:"call",id,frame:encodeHta([target,args])});if(cancelled)this.worker.postMessage({type:"cancel",id});}).catch(reject);
    });
  }
  releaseHandle(handle){if(handle.context!==this)throw new Error("hta/handle-owner-mismatch");const wireHandle=new HtaHandle(handle.owner,handle.type,handle.id);this.worker.postMessage({type:"release",frame:encodeHta(wireHandle)});}
  async createSession(name){await this.call("session/create",[name]);return this.session(name);}
  session(name="ROOT"){let session=this.sessions.get(name);if(!session){session=new HtaSession(this,name);this.sessions.set(name,session);}return session;}
  listSessions(){return this.call("session/list",[]);}
  filesystemForSession(name){return this.sessionFilesystems.get(name)??null;}
  async message(message) {
    if(message.type==="ready"){this.readyResolve();return;}if(message.type==="fatal"){this.fail(new Error(message.error?.message??"HTA worker failed"));return;}
    if(message.type==="result"){const pending=this.pending.get(message.id);if(!pending)return;this.pending.delete(message.id);const value=bindHandles(decodeHta(message.frame),this);message.ok?pending.resolve(value):pending.reject(errorFrom(value));return;}
    if(message.type==="host-call"){const key=`${message.service}/${message.method}`,handler=this.hostCalls[key],sessionId=message.session??"ROOT";try{if(!handler)throw new Error(`hta/host-call-denied: ${key}`);const value=await handler.call({context:this.session(sessionId),kernelContext:this,kernelId:this.kernelId??null,sessionId,task:message.task},...decodeHta(message.frame));this.worker.postMessage({type:"delivery",call:message.call,ok:true,frame:encodeHta(value)});}catch(error){this.worker.postMessage({type:"delivery",call:message.call,ok:false,frame:encodeHta(errorValue(error))});}}
  }
  fail(error){this.readyReject(error);for(const pending of this.pending.values())pending.reject(error);this.pending.clear();}
  close(){this.worker.postMessage({type:"close"});this.worker.terminate();}
}

export class HtaSession {
  constructor(context,name){if(typeof name!=="string"||!name.length)throw new Error("INVALID_SESSION_NAME");this.context=context;this.name=name;}
  call(target,args=[]){if(target==="eval")return this.eval(args[0]);if(target==="eval-bound")return this.evalBound(args[0],args[1]);if(target==="complete")return this.complete(args[0]);return this.context.call(target,args);}
  filesystemForSession(name){return this.context.filesystemForSession(name);}
  eval(source){return this.context.call("session/eval",[this.name,source]);}
  evalBound(source,bindings=[]){return this.context.call("session/eval-bound",[this.name,source,bindings]);}
  complete(prefix){return this.context.call("session/complete",[this.name,prefix]);}
  info(){return this.context.call("session/info",[this.name]);}
  async attachFilesystem(providerId){await this.context.call("session/attach-filesystem",[this.name,providerId]);this.context.sessionFilesystems.set(this.name,providerId);return true;}
  async close(){const result=await this.context.call("session/close",[this.name]);this.context.sessionFilesystems.delete(this.name);this.context.sessions.delete(this.name);return result;}
}

function bindHandles(value,context){if(value instanceof HtaHandle){value.context=context;const tag=context.handleTags[value.type];if(tag){value.displayTag=tag;value.displayKind=value.type;}return value;}if(Array.isArray(value)){value.forEach(item=>bindHandles(item,context));}else if(value instanceof Set){for(const item of value)bindHandles(item,context);}else if(value instanceof Map){for(const [key,item]of value){bindHandles(key,context);bindHandles(item,context);}}return value;}
function validateHandles(value,context){if(value instanceof HtaHandle){if(value.released)throw new Error("hta/handle-released");if(value.context!==context)throw new Error("hta/handle-owner-mismatch");return;}if(Array.isArray(value)){value.forEach(item=>validateHandles(item,context));}else if(value instanceof Set){for(const item of value)validateHandles(item,context);}else if(value instanceof Map){for(const [key,item]of value){validateHandles(key,context);validateHandles(item,context);}}}
function errorValue(error){return new Map([[new HtaKeyword("code"),new HtaKeyword("host/error")],[new HtaKeyword("message"),String(error?.message??error)],[new HtaKeyword("origin"),new HtaKeyword("browser")],[new HtaKeyword("retryable"),false]]);}
function errorFrom(value){if(value instanceof Error)return value;if(value instanceof Map){for(const[key,item]of value)if(key instanceof HtaKeyword&&key.name==="message")return new Error(String(item));}return new Error(String(value));}
