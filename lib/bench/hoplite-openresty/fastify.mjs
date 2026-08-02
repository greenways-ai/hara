import Fastify from "fastify";

const app = Fastify({ logger: false });
app.get("/hello", async (_, reply) => reply.type("text/plain").send("Hello from Fastify\n"));
app.get("/json", async () => ({ message: "Hello from Fastify" }));
app.get("/delay", async (_, reply) => {
  await new Promise((resolve) => setTimeout(resolve, 25));
  return reply.type("text/plain").send("delayed 25ms\n");
});
await app.listen({ host: "127.0.0.1", port: 18084 });
