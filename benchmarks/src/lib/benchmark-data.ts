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
  status: string;
  analysis?: { steady_ns?: number };
};
type LanguageEvidence = {
  measurements: LanguageMeasurement[];
  runtime_order?: string[];
  environment?: { timestamp?: string };
};
export type ComparisonRow = {
  runtime: string;
  display: string;
  overallRatio: number;
  haraWins: number;
  competitorWins: number;
};
export type ClassComparisonRow = ComparisonRow & {
  ratios: Record<string, number | null>;
};

const readJson = <T>(relativePath: string, fallback: T): T => {
  const path = fileURLToPath(new URL(relativePath, import.meta.url));
  return existsSync(path) ? JSON.parse(readFileSync(path, "utf8")) as T : fallback;
};

export const catalog = catalogSource;
const languageEvidence = readJson<LanguageEvidence>("../../data/language-shootout-reference.json", { measurements: [] });
const classEvidence = readJson<LanguageEvidence>("../../data/class-reference.json", { measurements: [] });
const runtimeEvidence = readJson<Evidence>("../../data/runtime.json", { measurements: [], environment: { timestamp: "" } });
const httpEvidence = readJson<Evidence>("../../data/http.json", { measurements: [], environment: { timestamp: "" } });

export const haraRuntime = "hara-rust-full";
export const haraClassRuntime = "hara-rust-whole-wasm-prepared";
export const runtimeBaseline = "hara-rust-full";
export const workloads = ["fib", "loop", "nested", "collatz", "tree", "binary-tree", "nqueens", "json-parse"] as const;
export type Workload = typeof workloads[number];

export const workloadMeta: Record<string, { label: string; description: string }> = {
  fib: { label: "Fibonacci", description: "Recursive integer calls." },
  loop: { label: "Loop", description: "Tight arithmetic loop." },
  nested: { label: "Nested", description: "Nested arithmetic and calls." },
  collatz: { label: "Collatz", description: "Branch-heavy integer iteration." },
  tree: { label: "Tree", description: "Recursive tree construction." },
  "binary-tree": { label: "Binary tree", description: "Binary tree allocation and traversal." },
  nqueens: { label: "N-Queens", description: "Backtracking search." },
  "json-parse": { label: "JSON", description: "JSON parsing and access." }
};

const displayNames: Record<string, string> = {
  "hara-rust-full": "Hara native full",
  "hara-rust-vm": "Hara native VM",
  "hara-wasm-full": "Hara WebAssembly full",
  "hara-wasm-vm": "Hara WebAssembly VM",
  "hara-wasm-core": "Hara WebAssembly core",
  "hara-truffle-full": "Hara Truffle full",
  "hara-truffle-vm": "Hara Truffle VM",
  "hara-jvm-full": "Hara JVM full",
  "hara-jvm-vm": "Hara JVM VM",
  "pypy-prepared": "PyPy",
  "node-prepared": "Node.js",
  "ruby-yjit-prepared": "Ruby YJIT",
  "clojure-prepared": "Clojure",
  "luajit-prepared": "LuaJIT",
  "sbcl-prepared": "SBCL",
  "chez-prepared": "Chez Scheme",
  "guile-prepared": "Guile",
  "bb-prepared": "Babashka",
  "rust-prepared": "Rust",
  "c-prepared": "C",
  "java-prepared": "Java",
  "python-prepared": "Python"
};

const measurementMap = new Map(languageEvidence.measurements.map((row) => [`${row.runtime}:${row.workload}`, row]));
const classMeasurementMap = new Map(classEvidence.measurements.map((row) => [`${row.runtime}:${row.workload}`, row]));
const runtimeMap = new Map(runtimeEvidence.measurements.map((row) => [`${row.runtime}:${row.workload}`, row]));

function steadyNs(row?: LanguageMeasurement): number | null {
  const value = row?.analysis?.steady_ns;
  return typeof value === "number" && Number.isFinite(value) && value > 0 ? value : null;
}

function geometricMean(values: number[]): number {
  const filtered = values.filter((value) => Number.isFinite(value) && value > 0);
  if (!filtered.length) return 0;
  return Math.exp(filtered.reduce((sum, value) => sum + Math.log(value), 0) / filtered.length);
}

export function ratioFor(runtime: string, workload: string): number | null {
  const hara = steadyNs(measurementMap.get(`${haraRuntime}:${workload}`));
  const competitor = steadyNs(measurementMap.get(`${runtime}:${workload}`));
  return hara && competitor ? competitor / hara : null;
}

export function ratioForClass(runtime: string, workload: string): number | null {
  const hara = steadyNs(classMeasurementMap.get(`${haraClassRuntime}:${workload}`));
  const competitor = steadyNs(classMeasurementMap.get(`${runtime}:${workload}`));
  return hara && competitor ? competitor / hara : null;
}

export const haraLanguageOverall = geometricMean(workloads.flatMap((workload: Workload) => {
  const value = ratioFor(haraRuntime, workload);
  return value ? [value] : [];
}));

export const comparisonRows: ComparisonRow[] = [
  ...catalog.language_competitors.map((name: string) => `${name}-prepared`)
].map((runtime: string) => {
  const ratios = workloads.flatMap((workload: Workload) => {
    const ratio = ratioFor(runtime, workload);
    return ratio ? [ratio] : [];
  });
  const haraWins = workloads.filter((workload: Workload) => (ratioFor(runtime, workload) ?? 0) > 1).length;
  const competitorWins = workloads.filter((workload: Workload) => {
    const ratio = ratioFor(runtime, workload);
    return ratio !== null && ratio < 1;
  }).length;
  return {
    runtime,
    display: displayNames[runtime] ?? runtime,
    overallRatio: geometricMean(ratios),
    haraWins,
    competitorWins
  };
});

export const runtimeRows = catalog.artifacts.map((artifact: { id: string; host: string; representation: string; summary: string; invocation: string }) => {
  const ratios = workloads.flatMap((workload: Workload) => {
    const baseline = runtimeMap.get(`${runtimeBaseline}:${workload}`)?.steady_ns;
    const value = runtimeMap.get(`${artifact.id}:${workload}`)?.steady_ns;
    return typeof baseline === "number" && baseline > 0 && typeof value === "number" && value > 0
      ? [value / baseline]
      : [];
  });
  return {
    ...artifact,
    overallRatio: geometricMean(ratios)
  };
});

function buildClassRows(runtimes: string[]): ClassComparisonRow[] {
  return runtimes.map((runtime: string) => {
    const ratios = Object.fromEntries(workloads.map((workload: Workload) => [workload, ratioForClass(runtime, workload)]));
    const overallRatio = geometricMean(workloads.flatMap((workload: Workload) => {
      const ratio = ratios[workload];
      return ratio ? [ratio] : [];
    }));
    const haraWins = workloads.filter((workload: Workload) => (ratioForClass(runtime, workload) ?? 0) > 1).length;
    const competitorWins = workloads.filter((workload: Workload) => {
      const ratio = ratioForClass(runtime, workload);
      return ratio !== null && ratio < 1;
    }).length;
    return {
      runtime,
      display: displayNames[runtime] ?? runtime,
      overallRatio,
      haraWins,
      competitorWins,
      ratios
    };
  });
}

export const classGroups = [
  {
    id: "dynamic",
    label: "Dynamic runtimes",
    description: "JIT and dynamic-language runtimes.",
    rows: buildClassRows(catalog.class_competitors.map((name: string) => `${name}-prepared`))
  },
  {
    id: "lisp",
    label: "Lisp family",
    description: "Lisp and Scheme implementations.",
    rows: buildClassRows(catalog.lisp_competitors.map((name: string) => `${name}-prepared`))
  },
  {
    id: "reference",
    label: "Native references",
    description: "Rust, C, Java and Python reference implementations.",
    rows: buildClassRows(catalog.reference_competitors.map((name: string) => `${name}-prepared`))
  }
];

export const httpRows = httpEvidence.http_measurements ?? [];
export const runtimeTimestamp = runtimeEvidence.environment.timestamp;
export const languageTimestamp = languageEvidence.environment?.timestamp ?? "";
export const classTimestamp = classEvidence.environment?.timestamp ?? "";
export const httpTimestamp = httpEvidence.environment.timestamp;

export function formatRatio(value: number | null | undefined): string {
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) return "—";
  return `${value.toFixed(value >= 10 ? 1 : 2)}×`;
}
