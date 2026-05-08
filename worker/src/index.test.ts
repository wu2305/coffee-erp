import { describe, expect, it } from "vitest";

import worker, { handleGetState, handlePutState, isAllowedOrigin, stateKey } from "./index";

type StoredValue = Record<string, unknown> | null;

class MockKvNamespace {
  private readonly values = new Map<string, string>();

  async get(key: string, type?: string): Promise<StoredValue | string | null> {
    const raw = this.values.get(key);
    if (raw === undefined) {
      return null;
    }
    if (type === "json") {
      return JSON.parse(raw) as StoredValue;
    }
    return raw;
  }

  async put(key: string, value: string): Promise<void> {
    this.values.set(key, value);
  }
}

function createEnv() {
  return {
    KV_NAMESPACE: new MockKvNamespace() as unknown as KVNamespace,
    ALLOWED_ORIGINS: "https://app.example.com,https://mirror.example.com",
  };
}

describe("stateKey", () => {
  it("builds_kv_key_with_store_id", () => {
    expect(stateKey("store-123")).toBe("coffee_erp:store:store-123:state");
  });
});

describe("isAllowedOrigin", () => {
  it("returns_true_for_allowed_origin", () => {
    expect(
      isAllowedOrigin("https://app.example.com", [
        "https://app.example.com",
        "https://mirror.example.com",
      ]),
    ).toBe(true);
  });

  it("returns_false_for_unlisted_origin", () => {
    expect(
      isAllowedOrigin("https://evil.example.com", [
        "https://app.example.com",
        "https://mirror.example.com",
      ]),
    ).toBe(false);
  });

  it("returns_true_when_origin_is_absent_or_allowlist_is_empty", () => {
    expect(isAllowedOrigin(null, ["https://app.example.com"])).toBe(true);
    expect(isAllowedOrigin("https://app.example.com", [])).toBe(true);
  });
});

describe("handleGetState", () => {
  it("returns_seed_state_when_kv_is_empty", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a");

    const response = await handleGetState(request, env);
    const payload = (await response.json()) as { state: Record<string, unknown> };

    expect(response.status).toBe(200);
    expect(payload.state.schema_version).toBe(1);
    expect(payload.state.revision).toBe(0);
    expect(payload.state.store).toEqual({
      id: "store-a",
      name: "Coffee ERP",
      water_tds: null,
    });
    expect(
      (payload.state.coffee_parameters as { bean_varieties: Array<{ label: string }> })
        .bean_varieties.map((item) => item.label),
    ).toEqual(["瑰夏/希爪种", "埃塞原生 (74系)", "波旁", "铁皮卡/卡杜拉/", "象豆种", "印尼咖啡"]);
    expect(
      (payload.state.coffee_parameters as { processing_methods: Array<{ label: string }> })
        .processing_methods.map((item) => item.label),
    ).toEqual(["日晒", "水洗", "蜜处理", "轻厌氧", "强厌氧", "增味"]);
    expect((payload.state.grinder_profiles as Array<{ name: string }>).map((item) => item.name)).toEqual([
      "Ditting",
    ]);
    expect((payload.state.brewing_plan_categories as Array<{ plans: unknown[] }>).map((item) => item.plans.length)).toEqual([
      2,
      1,
      1,
    ]);
    expect(payload.state.updated_at).toBe("2026-05-02T00:00:00Z");
  });

  it("uses_store_id_seed_when_query_store_id_is_absent", async () => {
    const env = {
      KV_NAMESPACE: new MockKvNamespace() as unknown as KVNamespace,
      STORE_ID_SEED: "store-from-env",
    };
    const request = new Request("https://api.example.com/api/state");

    const response = await handleGetState(request, env);
    const payload = (await response.json()) as { state: Record<string, unknown> };

    expect(response.status).toBe(200);
    expect(payload.state.store).toEqual({
      id: "store-from-env",
      name: "Coffee ERP",
      water_tds: null,
    });
  });
});

describe("handlePutState", () => {
  it("increments_revision_and_persists_state_when_revision_matches", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(baseState("store-a", 0, "Coffee ERP Store A", 80)),
    });

    const response = await handlePutState(request, env);
    const payload = (await response.json()) as { state: Record<string, unknown> };

    expect(response.status).toBe(200);
    expect(payload.state.revision).toBe(1);
    expect(payload.state.store).toEqual({
      id: "store-a",
      name: "Coffee ERP Store A",
      water_tds: 80,
    });

    const getResponse = await handleGetState(
      new Request("https://api.example.com/api/state?store_id=store-a"),
      env,
    );
    const getPayload = (await getResponse.json()) as { state: Record<string, unknown> };
    expect(getPayload.state.revision).toBe(1);
  });

  it("returns_conflict_when_revision_is_stale", async () => {
    const env = createEnv();
    const firstPutRequest = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(baseState("store-a", 0)),
    });
    await handlePutState(firstPutRequest, env);

    const stalePutRequest = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(baseState("store-a", 0)),
    });
    const response = await handlePutState(stalePutRequest, env);
    const payload = (await response.json()) as {
      error: { code: string; message: string; current_revision: number };
    };

    expect(response.status).toBe(409);
    expect(payload).toEqual({
      error: {
        code: "revision_conflict",
        message: "state revision is stale, refresh before retry",
        current_revision: 1,
      },
    });
  });

  it("rejects_invalid_app_state_payload", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "PUT",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        revision: 0,
        store: {
          id: "store-a",
          name: "Coffee ERP",
          water_tds: null,
        },
      }),
    });

    const response = await handlePutState(request, env);
    const payload = (await response.json()) as { error: { code: string; message: string } };

    expect(response.status).toBe(400);
    expect(payload).toEqual({
      error: {
        code: "invalid_state_payload",
        message: "request body must be a valid AppState object",
      },
    });
  });
});

describe("worker fetch CORS", () => {
  it("handles_options_preflight_for_allowed_origin", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "OPTIONS",
      headers: {
        Origin: "https://app.example.com",
      },
    });

    const response = await worker.fetch(request, env);

    expect(response.status).toBe(204);
    expect(response.headers.get("Access-Control-Allow-Origin")).toBe("https://app.example.com");
    expect(response.headers.get("Access-Control-Allow-Methods")).toBe("GET,PUT,OPTIONS");
    expect(response.headers.get("Access-Control-Allow-Headers")).toBe("Content-Type");
  });

  it("adds_allowed_origin_header_for_allowed_origin", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "GET",
      headers: {
        Origin: "https://app.example.com",
      },
    });

    const response = await worker.fetch(request, env);

    expect(response.status).toBe(200);
    expect(response.headers.get("Access-Control-Allow-Origin")).toBe("https://app.example.com");
    expect(response.headers.get("Vary")).toBe("Origin");
  });

  it("rejects_disallowed_origin_with_403", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "GET",
      headers: {
        Origin: "https://evil.example.com",
      },
    });

    const response = await worker.fetch(request, env);
    const payload = (await response.json()) as { error: { code: string; message: string } };

    expect(response.status).toBe(403);
    expect(payload).toEqual({
      error: {
        code: "forbidden_origin",
        message: "origin is not allowed by ALLOWED_ORIGINS",
      },
    });
  });

  it("returns_405_for_unsupported_method", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/state?store_id=store-a", {
      method: "POST",
    });

    const response = await worker.fetch(request, env);
    const payload = (await response.json()) as { error: { code: string; message: string } };

    expect(response.status).toBe(405);
    expect(payload).toEqual({
      error: {
        code: "method_not_allowed",
        message: "method must be GET, PUT, or OPTIONS",
      },
    });
  });

  it("returns_404_for_unknown_route", async () => {
    const env = createEnv();
    const request = new Request("https://api.example.com/api/missing", {
      method: "GET",
    });

    const response = await worker.fetch(request, env);
    const payload = (await response.json()) as { error: { code: string; message: string } };

    expect(response.status).toBe(404);
    expect(payload).toEqual({
      error: {
        code: "not_found",
        message: "route not found",
      },
    });
  });
});

function baseState(
  storeId: string,
  revision: number,
  storeName = "Coffee ERP",
  waterTds: number | null = null,
) {
  return {
    schema_version: 1,
    revision,
    store: {
      id: storeId,
      name: storeName,
      water_tds: waterTds,
    },
    coffee_parameters: {
      bean_varieties: [],
      roast_levels: [],
      processing_methods: [],
    },
    grinder_profiles: [],
    water_quality_adjustments: [],
    brewing_plan_categories: [],
    beans: [],
    roast_methods: [],
    roast_profiles: [],
    batches: [],
    updated_at: "2026-05-03T00:00:00Z",
  };
}
