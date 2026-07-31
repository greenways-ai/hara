import "fake-indexeddb/auto";
import { serveNodeProvider } from "@hara-lang/hta/provider/node";
import { callNoir } from "@hara-lang/noir";

const loaderUrl = new URL(/* @vite-ignore */ "../assets/noir-loader.js", import.meta.url).toString();
serveNodeProvider(
  (operation, args) => callNoir(loaderUrl, operation, args),
  { errorCode: "noir/error" }
);
