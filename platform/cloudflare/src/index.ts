import { route } from "./router";

export default {
  fetch(request, env): Promise<Response> {
    return route(request, env);
  },
} satisfies ExportedHandler<Env>;
