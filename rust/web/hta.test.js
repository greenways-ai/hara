import assert from "node:assert/strict";
import test from "node:test";
import { BrowserPromiseProvider, decodeHta, encodeHta, HtaContext, HtaHandle, HtaKeyword, loadHtaExtension, parseHtaManifest } from "./hta.js";

const tensorDescriptor='{:namespace "math.tensor" :version "1" :provider :wasm :module "tensor.wasm" :abi :hta.v1 :exports {"open" {:args [] :returns :value :async true}} :handles {"tensor" {:tag math}} :capabilities []}';

test("HTA1 browser codec matches the Java/Rust golden vector",()=>{assert.deepEqual([...encodeHta(["x",42,true])],[72,84,65,49,9,0,0,0,3,4,0,0,0,1,120,3,0,0,0,0,0,0,0,42,2]);assert.deepEqual(decodeHta(encodeHta(["x",42,true])),["x",42,true]);});
test("HTA1 floats preserve IEEE-754 values",()=>{for(const value of [0.28,-0,Infinity,-Infinity,NaN]){const decoded=decodeHta(encodeHta(value));if(Number.isNaN(value))assert.ok(Number.isNaN(decoded));else assert.ok(Object.is(decoded,value));}});
test("opaque handles round trip canonically",()=>{const value=new HtaHandle("runtime","cursor",42n);const decoded=decodeHta(encodeHta(value));assert.equal(decoded.owner,"runtime");assert.equal(decoded.type,"cursor");assert.equal(decoded.id,42n);assert.equal(decoded.toString(),"#ht[:handle 42]");});
test("canonical maps ignore insertion order",()=>{const a=new Map([[new HtaKeyword("b"),2],[new HtaKeyword("a"),1]]),b=new Map([[new HtaKeyword("a"),1],[new HtaKeyword("b"),2]]);assert.deepEqual(encodeHta(a),encodeHta(b));});
test("context applies registered public handle tags",async()=>{const worker=new FakeWorker();const context=new HtaContext({worker,moduleUrl:"runtime.wasm",handleTags:{tensor:"math"}});worker.emit({type:"ready"});const result=context.call("open",[]);await Promise.resolve();const call=worker.sent.find(message=>message.type==="call");worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(new HtaHandle("math.tensor","tensor",42n))});assert.equal(String(await result),"#math[:tensor 42]");context.close();});
test("manifest parser validates compact public tags",()=>{const manifest=parseHtaManifest(tensorDescriptor);assert.equal(manifest.namespace,"math.tensor");assert.equal(manifest.module,"tensor.wasm");assert.deepEqual(manifest.handleTags,{tensor:"math"});assert.throws(()=>parseHtaManifest(tensorDescriptor.replace(":tag math",":tag Math")),/invalid handle tag/);});
test("descriptor loader resolves wasm and applies handle tags",async()=>{const worker=new FakeWorker();const context=await loadHtaExtension({worker,descriptor:tensorDescriptor,packageUrl:"https://example.test/extensions/math/"});assert.equal(worker.sent[0].moduleUrl,"https://example.test/extensions/math/tensor.wasm");worker.emit({type:"ready"});const result=context.call("open",[]);await Promise.resolve();const call=worker.sent.find(message=>message.type==="call");worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(new HtaHandle("math.tensor","tensor",42n))});assert.equal(String(await result),"#math[:tensor 42]");context.close();});
test("descriptor loader fetches EDN when given its URL",async()=>{const worker=new FakeWorker(),descriptorUrl=`data:text/plain,${encodeURIComponent(tensorDescriptor)}`;const context=await loadHtaExtension({worker,descriptorUrl,moduleBytes:new Uint8Array()});assert.deepEqual(context.manifest.handleTags,{tensor:"math"});assert.ok(worker.sent[0].moduleBytes instanceof Uint8Array);context.close();});
test("context releases bound handles once and rejects later use",async()=>{const worker=new FakeWorker();const context=new HtaContext({worker,moduleUrl:"runtime.wasm"});worker.emit({type:"ready"});const result=context.call("open",[]);await Promise.resolve();const call=worker.sent.find(message=>message.type==="call");worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(new HtaHandle("runtime","cursor",42n))});const handle=await result;handle.release();handle.release();const releases=worker.sent.filter(message=>message.type==="release");assert.equal(releases.length,1);const released=decodeHta(releases[0].frame);assert.equal(released.id,42n);await assert.rejects(context.call("use",[handle]),/hta\/handle-released/);context.close();});
test("context exposes worker results as promises",async()=>{const worker=new FakeWorker();const context=new HtaContext({worker,moduleUrl:"runtime.wasm"});worker.emit({type:"ready"});const result=context.call("eval",["(+ 1 2)"]);await Promise.resolve();const call=worker.sent.find(message=>message.type==="call");worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(3)});assert.equal(await result,3);context.close();});
test("context cancellation is forwarded to its worker",async()=>{const worker=new FakeWorker();const context=new HtaContext({worker,moduleUrl:"runtime.wasm"});worker.emit({type:"ready"});const result=context.call("eval",["slow"]);const rejection=assert.rejects(result,/cancelled/);result.cancel();await Promise.resolve();await Promise.resolve();assert.equal(worker.sent.at(-1).type,"cancel");await rejection;context.close();});
test("context registers kernel-issued mounts and sessions attach numeric ids",async()=>{
  const worker=new FakeWorker(),events=[];
  const filesystemHost={register:async(_context,id,descriptor)=>events.push(["register",id,descriptor]),close:async(_context,id)=>events.push(["close",id])};
  const context=new HtaContext({worker,moduleUrl:"runtime.wasm",filesystemHost});
  worker.emit({type:"ready"});
  const creating=context.createFilesystem({provider:"memory"});
  await Promise.resolve();await Promise.resolve();
  let call=worker.sent.find(message=>message.type==="call");
  assert.equal(decodeHta(call.frame)[0],"filesystem/create");
  worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(7)});
  assert.equal(await creating,7);
  assert.deepEqual(events,[["register",7,{provider:"memory"}]]);
  const attaching=context.session("alpha").attachFilesystem(7);
  await Promise.resolve();await Promise.resolve();
  call=worker.sent.filter(message=>message.type==="call").at(-1);
  assert.deepEqual(decodeHta(call.frame),["session/attach-filesystem",["alpha",7]]);
  worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(true)});
  assert.equal(await attaching,true);
  const closing=context.closeFilesystem(7);
  await Promise.resolve();await Promise.resolve();
  call=worker.sent.filter(message=>message.type==="call").at(-1);
  worker.emit({type:"result",id:call.id,ok:true,frame:encodeHta(true)});
  assert.equal(await closing,true);
  assert.deepEqual(events.at(-1),["close",7]);
  context.close();
});

class FakeWorker{constructor(){this.listeners={};this.sent=[];}addEventListener(type,handler){this.listeners[type]=handler;}postMessage(message){this.sent.push(message);}emit(data){this.listeners.message({data});}terminate(){this.terminated=true;}}

test("browser promise provider uses native microtasks and ordered chaining",async()=>{
  const provider=new BrowserPromiseProvider(),events=[];
  const source=provider.run(()=>{events.push("run");return 20;});
  provider.then(source,value=>{events.push("first");return value+1;});
  const result=provider.then(source,value=>{events.push("second");return provider.run(()=>value*2);});
  events.push("sync");
  assert.equal(await result,40);
  assert.deepEqual(events,["sync","run","first","second"]);
});

test("browser promise provider adopts, recovers, finalizes, orders all, and settles once",async()=>{
  const provider=new BrowserPromiseProvider(),events=[];
  const adopted=provider.run(()=>provider.run(()=>7));
  const recovered=provider.catch(provider.run(()=>{throw new Error("broken");}),error=>error.message);
  const finalized=provider.finally(adopted,()=>{events.push("finally");});
  assert.deepEqual(await provider.all([recovered,finalized,3]),["broken",7,3]);
  assert.deepEqual(events,["finally"]);
  let resolveSource,rejectSource;
  const once=provider.create((resolve,reject)=>{resolveSource=resolve;rejectSource=reject;});
  assert.equal(resolveSource(1),true);assert.equal(rejectSource(new Error("late")),false);assert.equal(await once,1);
});

test("browser promise provider cancellation prevents deferred work",async()=>{
  const scheduled=[];
  const provider=new BrowserPromiseProvider({schedule:(task)=>{scheduled.push(task);return 0;},cancelSchedule:()=>scheduled.splice(0),enqueue:queueMicrotask});
  let ran=false;const delayed=provider.delay(10,()=>{ran=true;return 1;});
  assert.equal(delayed.cancel(),true);assert.equal(delayed.cancel(),false);
  await assert.rejects(delayed,/cancelled/);assert.equal(ran,false);assert.equal(scheduled.length,0);
});
