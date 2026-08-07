import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import catalogSource from "../../../core/lib/bench/catalog.json";

export type Measurement = {
  runtime: string;
  workload: string;
  prepare_ns: number;
  first_ns: number;
  steady_ns: number;
  throughput_per_sec: number;
};
export type HttpMeasurement = {
  server: string;
  route: string;
  status: string;
  requests_per_second?: number;
  mean_ms?: number;
  p50_ms?: number;
  p95_ms?: number;
  p99_ms?: number;
  reason?: string;
};
type Evidence = {
  measurements: Measurement[];
  environment: { timestamp: string };
  http_measurements?: HttpMeasurement[];
};
type LanguageMeasurement = {
  runtime: string;
  workload: string;
  steady_ns: number;
};
type LanguageEvidence = {
  measurements: LanguageMeasurement[];
  runtime_order?: string[];
  environment: { timestamp: string };
};
type RuntimeArtifact = {
  id: string;
  host: string;
  representation: string;
  summary: string;
  invocation: string;
};
type BenchmarkCatalog = {
  artifacts: RuntimeArtifact[];
  class_competitors: string[];
  language_competitors: string[];
  lisp_competitors: string[];
  reference_competitors: string[];
};

const catalog = catalogSource as BenchmarkCatalog;

const classEvidencePath = fileURLToPath(new URL("../../../core/lib/bench/results/class-reference.json", import.meta.url));
const runtimeEvidencePath = fileURLToPath(new URL("../../../core/lib/bench/results/runtime.json", import.meta.url));
const httpEvidencePath = fileURLToPath(new URL("../../../core/lib/bench/results/http.json", import.meta.url));
const languageEvidencePath = fileURLToPath(new URL("../../../core/lib/bench/results/language.json", import.meta.url));

function readEvidence<T>(path: string, fallback: T): T {
  if (!existsSync(path)) return fallback;
  return JSON.parse(readFileSync(path, "utf8")) as T;
}

const runtimeEvidence = readEvidence<Evidence>(runtimeEvidencePath, {
  measurements: [],
  environment: { timestamp: "unavailable" },
});
const httpEvidence = readEvidence<Evidence>(httpEvidencePath, {
  measurements: [],
  environment: { timestamp: "unavailable" },
  http_measurements: [],
});
const languageEvidence = readEvidence<LanguageEvidence>(languageEvidencePath, {
  measurements: [],
  environment: { timestamp: "unavailable" },
});
const classEvidence = readEvidence<LanguageEvidence>(classEvidencePath, {
  measurements: [],
  environment: { timestamp: "unavailable" },
});

const runtimeBaseline = "hara-rust-full";
const haraRuntime = "hara-rust-full";
const haraClassRuntime = "hara-rust-whole-wasm-prepared";

const workloadMeta: Record<string, { label: string; summary: string }> = {
  fib: { label: "Fibonacci", summary: "Recursive integer calls" },
  json: { label: "JSON", summary: "Parse and traverse structured data" },
  loops: { label: "Loops", summary: "Tight integer iteration" },
  sieve: { label: "Sieve", summary: "Prime-number array workload" },
  sort: { label: "Sort", summary: "Comparison and mutation" },
  strings: { label: "Strings", summary: "Text construction and traversal" },
  records: { label: "Records", summary: "Object allocation and field access" },
  dispatch: { label: "Dispatch", summary: "Dynamic method dispatch" },
};

const runtimeLabels: Record<string, string> = {
  "luajit-prepared": "LuaJIT",
  "pypy-prepared": "PyPy",
  "node-prepared": "Node.js",
  "ruby-yjit-prepared": "Ruby YJIT",
  "clojure-prepared": "Clojure",
  "sbcl-prepared": "SBCL",
  "chez-prepared": "Chez Scheme",
  "guile-prepared": "Guile",
  "bb-prepared": "Babashka",
  "rust-prepared": "Rust",
  "c-prepared": "C",
  "java-prepared": "Java",
  "python-prepared": "Python",
};

const runtimeMeasurements = new Map(
  runtimeEvidence.measurements.map((measurement) => [`${measurement.runtime}:${measurement.workload}`, measurement]),
);
const languageMeasurements = new Map(
  languageEvidence.measurements.map((measurement) => [`${measurement.runtime}:${measurement.workload}`, measurement]),
);
const classMeasurements = new Map(
  classEvidence.measurements.map((measurement) => [`${measurement.runtime}:${measurement.workload}`, measurement]),
);

const workloads = [...new Set([
  ...runtimeEvidence.measurements.map((measurement) => measurement.workload),
  ...languageEvidence.measurements.map((measurement) => measurement.workload),
  ...classEvidence.measurements.map((measurement) => measurement.workload),
])].sort();

function geometricMean(values: number[]) {
  const valid = values.filter((value) => Number.isFinite(value) && value > 0);
  if (!valid.length) return null;
  return Math.exp(valid.reduce((total, value) => total + Math.log(value), 0) / valid.length);
}

function ratioFor(runtime: string, workload: string) {
  const hara = languageMeasurements.get(`${haraRuntime}:${workload}`)?.steady_ns;
  const competitor = languageMeasurements.get(`${runtime}:${workload}`)?.steady_ns;
  if (!hara || !competitor) return null;
  return competitor / hara;
}

const languageRuntimeIds = [
  haraRuntime,
  ...catalog.language_competitors.map((name: string) => `${name}-prepared`),
];

const languageRows = languageRuntimeIds.map((runtime) => ({
  runtime,
  label: runtime === haraRuntime ? "Hara" : (runtimeLabels[runtime] ?? runtime),
  overall: runtime === haraRuntime
    ? 1
    : geometricMean(workloads.flatMap((workload: string) => {
      const ratio = ratioFor(runtime, workload);
      return ratio == null ? [] : [ratio];
    })),
  cells: workloads.map((workload: string) => ({ workload, ratio: runtime === haraRuntime ? 1 : ratioFor(runtime, workload) })),
  haraWins: runtime === haraRuntime ? workloads.length : workloads.filter((workload: string) => (ratioFor(runtime, workload) ?? 0) > 1).length,
  competitorWins: runtime === haraRuntime ? 0 : workloads.filter((workload: string) => {
    const ratio = ratioFor(runtime, workload);
    return ratio != null && ratio < 1;
  }).length,
}));

function ratioForClass(runtime: string, workload: string) {
  const hara = classMeasurements.get(`${haraClassRuntime}:${workload}`)?.steady_ns;
  const competitor = classMeasurements.get(`${runtime}:${workload}`)?.steady_ns;
  if (!hara || !competitor) return null;
  return competitor / hara;
}

function buildClassRows(runtimeIds: string[]) {
  return [haraClassRuntime, ...runtimeIds].map((runtime) => {
    const isHara = runtime === haraClassRuntime;
    const overallRatio = isHara
      ? 1
      : geometricMean(workloads.flatMap((workload: string) => {
        const ratio = ratioForClass(runtime, workload);
        return ratio == null ? [] : [ratio];
      }));
    const haraWins = isHara ? workloads.length : workloads.filter((workload: string) => (ratioForClass(runtime, workload) ?? 0) > 1).length;
    const competitorWins = isHara ? 0 : workloads.filter((workload: string) => {
      const ratio = ratioForClass(runtime, workload);
      return ratio != null && ratio < 1;
    }).length;
    return {
      runtime,
      label: isHara ? "Hara" : (runtimeLabels[runtime] ?? runtime),
      overall: overallRatio,
      cells: workloads.map((workload: string) => ({ workload, ratio: isHara ? 1 : ratioForClass(runtime, workload) })),
      haraWins,
      competitorWins,
    };
  });
}

const classGroups = [
  {
    id: "class",
    title: "Dynamic language class",
    summary: "Hara against modern dynamic language runtimes.",
    rows: buildClassRows(catalog.class_competitors.map((name: string) => `${name}-prepared`)),
  },
  {
    id: "lisp",
    title: "Lisp class",
    summary: "Hara against established Lisp and Scheme systems.",
    rows: buildClassRows(catalog.lisp_competitors.map((name: string) => `${name}-prepared`)),
  },
  {
    id: "reference",
    title: "Reference systems",
    summary: "Hara against Rust, C, Java and Python reference implementations.",
    rows: buildClassRows(catalog.reference_competitors.map((name: string) => `${name}-prepared`)),
  },
];

export {
  catalog,
  classEvidence,
  classGroups,
  haraClassRuntime,
  haraRuntime,
  httpEvidence,
  languageEvidence,
  languageRows,
  runtimeBaseline,
  runtimeEvidence,
  runtimeLabels,
  runtimeMeasurements,
  workloadMeta,
  workloads,
};
