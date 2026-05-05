export interface Env {}

export default {
  async fetch(request: Request): Promise<Response> {
    const { pathname } = new URL(request.url);

    if (pathname.startsWith("/api/")) {
      return Response.json(
        {
          message: "Coffee ERP API placeholder",
          status: "not_implemented",
        },
        { status: 501 },
      );
    }

    return new Response("Coffee ERP worker placeholder", { status: 200 });
  },
} satisfies ExportedHandler<Env>;
