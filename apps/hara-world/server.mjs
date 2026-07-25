import { createServer as createHttpServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { WebSocketServer } from "ws";

const ROOT = fileURLToPath(new URL(".", import.meta.url));
const PUBLIC = join(ROOT, "src");
const RUNTIME = join(ROOT, "runtime");
const ROOM = /^[A-Za-z0-9_-]{16,64}$/;
const PEER = /^[A-Za-z0-9_-]{8,64}$/;
const TYPES = new Set(["offer", "answer", "candidate"]);

const MIME = {
  ".css": "text/css; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".svg": "image/svg+xml",
  ".wasm": "application/wasm"
};

function send(socket, message) {
  if (socket.readyState === socket.OPEN) socket.send(JSON.stringify(message));
}

export function createWorldServer({ host = "127.0.0.1", port = 8787 } = {}) {
  const rooms = new Map();
  const http = createHttpServer(async (request, response) => {
    try {
      const url = new URL(request.url ?? "/", `http://${request.headers.host}`);
      const runtime = url.pathname.startsWith("/runtime/");
      const relative = runtime
        ? url.pathname.slice("/runtime/".length)
        : url.pathname === "/" ? "index.html" : url.pathname.slice(1);
      const base = runtime ? RUNTIME : PUBLIC;
      const path = normalize(join(base, relative));
      if (!path.startsWith(`${base}/`)) throw new Error("invalid path");
      const info = await stat(path);
      if (!info.isFile()) throw new Error("not a file");
      response.writeHead(200, {
        "content-type": MIME[extname(path)] ?? "application/octet-stream",
        "cache-control": runtime ? "public, max-age=3600" : "no-cache",
        "cross-origin-opener-policy": "same-origin",
        "cross-origin-resource-policy": "same-origin"
      });
      response.end(await readFile(path));
    } catch {
      response.writeHead(404, { "content-type": "text/plain; charset=utf-8" });
      response.end("Not found");
    }
  });

  const sockets = new WebSocketServer({ noServer: true });
  http.on("upgrade", (request, socket, head) => {
    const url = new URL(request.url ?? "/", `http://${request.headers.host}`);
    if (url.pathname !== "/signal") {
      socket.destroy();
      return;
    }
    sockets.handleUpgrade(request, socket, head, client => sockets.emit("connection", client));
  });

  sockets.on("connection", socket => {
    let membership = null;

    socket.on("message", bytes => {
      let message;
      try {
        message = JSON.parse(String(bytes));
      } catch {
        send(socket, { type: "error", code: "message-invalid" });
        return;
      }

      if (message.type === "join") {
        if (membership || !ROOM.test(message.room ?? "") || !PEER.test(message.peer ?? "")) {
          send(socket, { type: "error", code: "join-invalid" });
          return;
        }
        const room = rooms.get(message.room) ?? new Map();
        if (room.size >= 2 || room.has(message.peer)) {
          send(socket, { type: "error", code: "room-full" });
          return;
        }
        membership = { room: message.room, peer: message.peer };
        rooms.set(message.room, room);
        room.set(message.peer, socket);
        send(socket, {
          type: "joined",
          room: message.room,
          peer: message.peer,
          peers: [...room.keys()].filter(peer => peer !== message.peer)
        });
        for (const [peer, other] of room) {
          if (peer !== message.peer) send(other, { type: "peer-joined", peer: message.peer });
        }
        return;
      }

      if (message.type === "signal" && membership) {
        const room = rooms.get(membership.room);
        const target = room?.get(message.to);
        const signal = message.signal;
        if (!target || !signal || !TYPES.has(signal.type)) {
          send(socket, { type: "error", code: "signal-invalid" });
          return;
        }
        send(target, { type: "signal", from: membership.peer, signal });
        return;
      }

      send(socket, { type: "error", code: "message-unsupported" });
    });

    socket.on("close", () => {
      if (!membership) return;
      const room = rooms.get(membership.room);
      room?.delete(membership.peer);
      if (!room?.size) {
        rooms.delete(membership.room);
      } else {
        for (const other of room.values()) {
          send(other, { type: "peer-left", peer: membership.peer });
        }
      }
    });
  });

  return {
    http,
    rooms,
    async listen() {
      await new Promise((resolve, reject) => {
        http.once("error", reject);
        http.listen(port, host, resolve);
      });
      return http.address();
    },
    async close() {
      for (const socket of sockets.clients) socket.close();
      await new Promise(resolve => sockets.close(resolve));
      await new Promise(resolve => http.close(resolve));
    }
  };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const port = Number(process.env.PORT ?? 8787);
  const server = createWorldServer({ host: process.env.HOST ?? "127.0.0.1", port });
  const address = await server.listen();
  console.log(`Hara World listening on http://${address.address}:${address.port}`);
}
