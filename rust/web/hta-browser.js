import { HtaContext } from "./hta.js";
const bytes=new Uint8Array(await (await fetch("/rust/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm")).arrayBuffer());
const worker=new Worker("./hta-worker.js",{type:"module"});
const context=new HtaContext({worker,moduleBytes:bytes,hostCalls:{"crypto.hash.sha256/digest":async input=>new Uint8Array(await crypto.subtle.digest("SHA-256",input))}});
window.htaContext=context;
window.htaSmoke=context.call("eval",['(ns user (:require [std.foundation.host :as host])) (+ 10 (count (deref (host/call "crypto.hash.sha256" "digest" (bytes 97 98 99)))))']);
