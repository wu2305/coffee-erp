const STATE_KEY_PREFIX = "coffee_erp:store:";
const STATE_KEY_SUFFIX = ":state";
const DEFAULT_STORE_ID = "store-default";
const DEFAULT_UPDATED_AT = "2026-05-02T00:00:00Z";

type AppStateRecord = {
  schema_version: number;
  revision: number;
  store: {
    id: string;
    name: string;
    water_tds: number | null;
  };
  coffee_parameters: {
    bean_varieties: unknown[];
    roast_levels: unknown[];
    processing_methods: unknown[];
  };
  grinder_profiles: unknown[];
  water_quality_adjustments: unknown[];
  brewing_plan_categories: unknown[];
  beans: unknown[];
  roast_methods: unknown[];
  roast_profiles: unknown[];
  batches: unknown[];
  updated_at: string;
  [key: string]: unknown;
};

type ErrorPayload = {
  error: {
    code: string;
    message: string;
    current_revision?: number;
  };
};

export interface Env {
  KV_NAMESPACE: KVNamespace;
  ALLOWED_ORIGINS?: string;
  STORE_ID_SEED?: string;
}

export function stateKey(storeId: string): string {
  return `${STATE_KEY_PREFIX}${storeId}${STATE_KEY_SUFFIX}`;
}

export function isAllowedOrigin(origin: string | null, allowedOrigins: string[]): boolean {
  if (origin === null) {
    return true;
  }
  if (allowedOrigins.length === 0) {
    return true;
  }
  return allowedOrigins.includes(origin);
}

export async function handleGetState(request: Request, env: Env): Promise<Response> {
  const storeId = resolveStoreId(request, env);
  const key = stateKey(storeId);
  const existing = await env.KV_NAMESPACE.get(key, "json");
  const state = isAppStateRecord(existing) ? existing : seedState(storeId);
  return Response.json({ state });
}

export async function handlePutState(request: Request, env: Env): Promise<Response> {
  const storeId = resolveStoreId(request, env);
  const key = stateKey(storeId);
  const payload = await request.json<unknown>().catch(() => null);
  if (!isAppStateRecord(payload)) {
    return jsonError(400, "invalid_state_payload", "request body must be a valid AppState object");
  }

  const existing = await env.KV_NAMESPACE.get(key, "json");
  const currentState = isAppStateRecord(existing) ? existing : seedState(storeId);

  if (payload.revision !== currentState.revision) {
    return jsonError(409, "revision_conflict", "state revision is stale, refresh before retry", {
      current_revision: currentState.revision,
    });
  }

  const now = new Date().toISOString();
  const nextState: AppStateRecord = {
    ...payload,
    revision: currentState.revision + 1,
    store: {
      ...payload.store,
      id: storeId,
    },
    updated_at: now,
  };
  await env.KV_NAMESPACE.put(key, JSON.stringify(nextState));
  return Response.json({ state: nextState });
}

function seedState(storeId: string): AppStateRecord {
  return {
    schema_version: 1,
    revision: 0,
    store: {
      id: storeId,
      name: "Coffee ERP",
      water_tds: null,
    },
    coffee_parameters: {
      bean_varieties: [
        catalogOption("bean-var-geisha", "瑰夏/希爪种", 1),
        catalogOption("bean-var-ethiopian-heirloom", "埃塞原生 (74系)", 2),
        catalogOption("bean-var-bourbon", "波旁", 3),
        catalogOption("bean-var-typica-caturra", "铁皮卡/卡杜拉/", 4),
        catalogOption("bean-var-maragogype", "象豆种", 5),
        catalogOption("bean-var-indonesian", "印尼咖啡", 6),
      ],
      roast_levels: [
        roastLevel("roast-level-very-light", "极浅", "95+", null, null, 1),
        roastLevel("roast-level-light", "浅", "90-95", 90, 95, 2),
        roastLevel("roast-level-light-medium", "浅中", "80-90", 80, 90, 3),
        roastLevel("roast-level-medium", "中", "70-80", 70, 80, 4),
        roastLevel("roast-level-medium-dark", "中深", "60-70", 60, 70, 5),
        roastLevel("roast-level-dark", "深", "50-60", 50, 60, 6),
      ],
      processing_methods: [
        catalogOption("process-sun-dried", "日晒", 1),
        catalogOption("process-washed", "水洗", 2),
        catalogOption("process-honey", "蜜处理", 3),
        catalogOption("process-light-anaerobic", "轻厌氧", 4),
        catalogOption("process-strong-anaerobic", "强厌氧", 5),
        catalogOption("process-flavor-enhanced", "增味", 6),
      ],
    },
    grinder_profiles: [
      {
        id: "grinder-ditting",
        name: "Ditting",
        notes: null,
        archived: false,
      },
    ],
    water_quality_adjustments: [
      waterQualityAdjustment(40, 60, 0, 0, "TDS 40-60"),
      waterQualityAdjustment(60, 80, -1, 0, "TDS 60-80"),
      waterQualityAdjustment(80, 100, 0, 0.1, "TDS 80-100"),
      waterQualityAdjustment(100, 150, -1, 0.1, "TDS 100-150"),
      waterQualityAdjustment(150, null, -2, 0.2, "TDS 150+"),
    ],
    brewing_plan_categories: [
      {
        id: "category-layered-flavor",
        name: "强层次，强风味",
        sort_order: 1,
        plans: [
          brewingPlan(
            "plan-cone-one-pour",
            "锥形滤杯一刀流",
            [
              matchAttr("ProcessingMethod", "process-washed"),
              matchAttr("ProcessingMethod", "process-light-anaerobic"),
            ],
            planParameters(2, "RF", "grinder-ditting", 1, 16, 16),
            ageFitting(5.5, 96, 6, 94),
            1,
          ),
          brewingPlan(
            "plan-standard-three-stage",
            "标准三段式",
            [matchAttr("ProcessingMethod", "process-washed")],
            planParameters(3, "山文", "grinder-ditting", 1, 16, 16),
            ageFitting(7.1, 93, 7.5, 91),
            2,
          ),
        ],
        archived: false,
      },
      {
        id: "category-strong-sweetness",
        name: "强甜感",
        sort_order: 2,
        plans: [
          brewingPlan(
            "plan-cake-one-pour",
            "蛋糕滤杯一刀流",
            [
              matchAttr("ProcessingMethod", "process-sun-dried"),
              matchAttr("ProcessingMethod", "process-light-anaerobic"),
              matchAttr("ProcessingMethod", "process-strong-anaerobic"),
            ],
            planParameters(2, "马赫", "grinder-ditting", 1, 15, 16),
            ageFitting(6.5, 93, 7, 91),
            1,
          ),
        ],
        archived: false,
      },
      {
        id: "category-dark-roast",
        name: "深烘",
        sort_order: 3,
        plans: [
          brewingPlan(
            "plan-volcano",
            "火山冲煮",
            [
              matchAttr("RoastLevel", "roast-level-dark"),
              matchAttr("BeanVariety", "bean-var-indonesian"),
            ],
            planParameters(1, "马赫", "grinder-ditting", 1, 13, 16),
            ageFitting(8.5, 85, 8.8, 85),
            1,
          ),
        ],
        archived: false,
      },
    ],
    beans: [],
    roast_methods: [],
    roast_profiles: [],
    batches: [],
    updated_at: DEFAULT_UPDATED_AT,
  };
}

function catalogOption(id: string, label: string, sortOrder: number) {
  return {
    id,
    label,
    sort_order: sortOrder,
    archived: false,
  };
}

function roastLevel(
  id: string,
  label: string,
  agtronRange: string,
  agtronMin: number | null,
  agtronMax: number | null,
  sortOrder: number,
) {
  return {
    id,
    label,
    agtron_range: agtronRange,
    agtron_min: agtronMin,
    agtron_max: agtronMax,
    sort_order: sortOrder,
    archived: false,
  };
}

function matchAttr(kind: "BeanVariety" | "ProcessingMethod" | "RoastLevel", optionId: string) {
  return {
    kind,
    option_id: optionId,
  };
}

function planParameters(
  pourStages: number,
  dripper: string,
  grinderProfileId: string,
  coffee: number,
  water: number,
  defaultDoseG: number,
) {
  return {
    pour_stages: pourStages,
    dripper,
    grinder_profile_id: grinderProfileId,
    ratio: {
      coffee,
      water,
    },
    default_dose_g: defaultDoseG,
  };
}

function ageFitting(
  day0GrindSize: number,
  day0WaterTempC: number,
  day14GrindSize: number,
  day14WaterTempC: number,
) {
  return {
    day0: {
      grind_size: day0GrindSize,
      water_temp_c: day0WaterTempC,
    },
    day14: {
      grind_size: day14GrindSize,
      water_temp_c: day14WaterTempC,
    },
  };
}

function brewingPlan(
  id: string,
  name: string,
  matchingAttributes: Array<ReturnType<typeof matchAttr>>,
  parameters: ReturnType<typeof planParameters>,
  ageFittingValue: ReturnType<typeof ageFitting>,
  priority: number,
) {
  return {
    id,
    name,
    matching_attributes: matchingAttributes,
    parameters,
    age_fitting: ageFittingValue,
    instructions: null,
    priority,
    archived: false,
  };
}

function waterQualityAdjustment(
  tdsMin: number | null,
  tdsMax: number | null,
  tempModC: number,
  grindMod: number,
  label: string,
) {
  return {
    tds_min: tdsMin,
    tds_max: tdsMax,
    temp_mod_c: tempModC,
    grind_mod: grindMod,
    label,
  };
}

function parseAllowedOrigins(raw: string | undefined): string[] {
  if (!raw) {
    return [];
  }
  return raw
    .split(",")
    .map((item) => item.trim())
    .filter((item) => item.length > 0);
}

function resolveStoreId(request: Request, env: Env): string {
  const url = new URL(request.url);
  const fromQuery = url.searchParams.get("store_id")?.trim();
  if (fromQuery && fromQuery.length > 0) {
    return fromQuery;
  }
  const fromEnv = env.STORE_ID_SEED?.trim();
  if (fromEnv && fromEnv.length > 0) {
    return fromEnv;
  }
  return DEFAULT_STORE_ID;
}

function withCors(response: Response, request: Request, env: Env): Response {
  const allowedOrigins = parseAllowedOrigins(env.ALLOWED_ORIGINS);
  const origin = request.headers.get("Origin");
  const headers = new Headers(response.headers);
  if (origin && isAllowedOrigin(origin, allowedOrigins)) {
    headers.set("Access-Control-Allow-Origin", origin);
    headers.set("Vary", "Origin");
  }
  headers.set("Access-Control-Allow-Methods", "GET,PUT,OPTIONS");
  headers.set("Access-Control-Allow-Headers", "Content-Type");
  return new Response(response.body, {
    status: response.status,
    statusText: response.statusText,
    headers,
  });
}

function jsonError(
  status: number,
  code: string,
  message: string,
  extra?: Partial<ErrorPayload["error"]>,
): Response {
  const payload: ErrorPayload = {
    error: {
      code,
      message,
      ...extra,
    },
  };
  return Response.json(payload, { status });
}

function isAppStateRecord(value: unknown): value is AppStateRecord {
  if (!isRecord(value)) {
    return false;
  }
  if (typeof value.schema_version !== "number" || typeof value.revision !== "number") {
    return false;
  }
  if (!isRecord(value.store)) {
    return false;
  }
  if (typeof value.store.id !== "string" || typeof value.store.name !== "string") {
    return false;
  }
  if (value.store.water_tds !== null && typeof value.store.water_tds !== "number") {
    return false;
  }
  if (!isRecord(value.coffee_parameters)) {
    return false;
  }
  if (!Array.isArray(value.coffee_parameters.bean_varieties)) {
    return false;
  }
  if (!Array.isArray(value.coffee_parameters.roast_levels)) {
    return false;
  }
  if (!Array.isArray(value.coffee_parameters.processing_methods)) {
    return false;
  }
  return (
    Array.isArray(value.grinder_profiles) &&
    Array.isArray(value.water_quality_adjustments) &&
    Array.isArray(value.brewing_plan_categories) &&
    Array.isArray(value.beans) &&
    Array.isArray(value.roast_methods) &&
    Array.isArray(value.roast_profiles) &&
    Array.isArray(value.batches) &&
    typeof value.updated_at === "string"
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);
    if (url.pathname !== "/api/state") {
      return withCors(jsonError(404, "not_found", "route not found"), request, env);
    }

    const allowedOrigins = parseAllowedOrigins(env.ALLOWED_ORIGINS);
    const origin = request.headers.get("Origin");
    if (!isAllowedOrigin(origin, allowedOrigins)) {
      return withCors(
        jsonError(403, "forbidden_origin", "origin is not allowed by ALLOWED_ORIGINS"),
        request,
        env,
      );
    }

    if (request.method === "OPTIONS") {
      return withCors(new Response(null, { status: 204 }), request, env);
    }
    if (request.method === "GET") {
      return withCors(await handleGetState(request, env), request, env);
    }
    if (request.method === "PUT") {
      return withCors(await handlePutState(request, env), request, env);
    }

    return withCors(
      jsonError(405, "method_not_allowed", "method must be GET, PUT, or OPTIONS"),
      request,
      env,
    );
  },
} satisfies ExportedHandler<Env>;
