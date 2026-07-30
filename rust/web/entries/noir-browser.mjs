import { serveBrowserProvider } from "@hara-lang/hta/provider/browser";
import { callNoir } from "@hara-lang/noir";

const loaderUrl = new URL(/* @vite-ignore */ "../assets/noir-loader.js", import.meta.url).toString();
serveBrowserProvider(
  (operation, args) => callNoir(loaderUrl, operation, args),
  { errorCode: "noir/error" }
);
