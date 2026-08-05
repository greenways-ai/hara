import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import catalogSource from "../../../lib/bench/catalog.json";

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
  label: string;
  overallRatio: number;
  haraWins: number;
  competitorWins: number;
};

type Catalog = typeof catalogSource;
export const catalog: Catalog = catalogSource;
const evidencePath = fileURLToPath(new URL("../../../lib/bench/results/reference-v2.json", import.meta.url));
const languageEvidencePath = fileURLToPath(new URL("../../../lib/bench/results/language-reference.json", import.meta.url));
export const evidence: Evidence | null = existsSync(evidencePath)
  ? JSON.parse(readFileSync(evidencePath, "utf8"))
  : null;
const languageEvidence: LanguageEvidence | null = existsSync(languageEvidencePath)
  ? JSON.parse(readFileSync(languageEvidencePath, "utf8"))
  : null;

const classEvidencePath = fileURLToPath(new URL("../../../lib/bench/results/class-reference.json", import.meta.url));
const classEvidence: LanguageEvidence | null = existsSync(classEvidencePath)
  ? JSON.parse(readFileSync(classEvidencePath, "utf8"))
  : null;

export const haraRuntime = "hara-rust-full";
export const haraClassRuntime = "hara-rust-whole-wasm-prepared";
export const workloads = catalog.corpus.workloads;
export const runtimeLabels: Record<string, string> = {
  "hara-rust-full": "Hara",
  "sbcl-prepared": "SBCL",
  "chez-prepared": "Chez Scheme",
  "guile-prepared": "Guile",
  "bb-prepared": "Babashka",
  "python-prepared": "Python",
  "c-prepared": "C",
  "java-prepared": "Java",
  "luajit-prepared": "LuaJIT",
  "pypy-prepared": "PyPy",
  "node-prepared": "Node",
  "ruby-yjit-prepared": "Ruby (YJIT)",
  "clojure-prepared": "Clojure",
  "hara-rust-whole-wasm-prepared": "Hara"
};
export const workloadMeta: Record<string, { label: string; summary: string }> = {
  "sieve-array": { label: "Sieve", summary: "Integer loops and mutable array traversal." },
  "towers-recursive": { label: "Towers", summary: "Recursive calls and stack movement." },
  "queens-backtracking": { label: "Queens", summary: "Backtracking, branching, and board checks." },
  "heap-permute": { label: "Heap permutation", summary: "Recursive permutation with repeated swaps." },
  "ackermann-deep": { label: "Ackermann", summary: "Very deep recursive call pressure." },
  "tak-branching": { label: "Tak", summary: "Branch-heavy recursive evaluation." },
  "collatz-range": { label: "Collatz", summary: "Integer loops, conditions, and range traversal." },
  "matrix-multiply": { label: "Matrix multiply", summary: "Nested numeric loops over array data." }
};

export const runtimeIndex = new Map<string, Measurement>(
  (evidence?.measurements ?? []).map((row) => [`${row.runtime}/${row.workload}`, row])
);
const languageIndex = new Map<string, number>(
  (languageEvidence?.measurements ?? [])
    .filter((row) => row.status === "ok" && row.analysis?.steady_ns != null)
    .map((row) => [`${row.runtime}/${row.workload}`, row.analysis!.steady_ns!])
);
const geometricMean = (values: number[]) =>
  values.length ? Math.exp(values.reduce((sum, value) => sum + Math.log(value), 0) / values.length) : null;
export const ratioFor = (runtime: string, workload: string) => {
  const hara = languageIndex.get(`${haraRuntime}/${workload}`);
  const candidate = languageIndex.get(`${runtime}/${workload}`);
  return hara && candidate ? candidate / hara : null;
};
const overallRatioFor = (runtime: string) =>
  geometricMean(workloads.flatMap((workload) => {
    const ratio = ratioFor(runtime, workload);
    return ratio == null ? [] : [ratio];
  }));

const languageRuntimes = languageEvidence?.runtime_order ?? [
  haraRuntime,
  ...catalog.language_competitors.map((name) => `${name}-prepared`)
];
export const comparisonRows: ComparisonRow[] = languageRuntimes
  .filter((runtime) => runtime !== haraRuntime)
  .flatMap((runtime) => {
    const overallRatio = overallRatioFor(runtime);
    if (overallRatio == null) return [];
    const haraWins = workloads.filter((workload) => (ratioFor(runtime, workload) ?? 0) > 1).length;
    const competitorWins = workloads.filter((workload) => {
      const ratio = ratioFor(runtime, workload);
      return ratio != null && ratio < 1;
    }).length;
    return [{
      runtime,
      label: runtimeLabels[runtime] ?? runtime.replace(/-prepared$/, ""),
      overallRatio,
      haraWins,
      competitorWins
    }];
  })
  .sort((left, right) => right.overallRatio - left.overallRatio);
export const haraLeadingRows = comparisonRows.filter((row) => row.overallRatio > 1);
export const competitorLeadingRows = comparisonRows.filter((row) => row.overallRatio < 1);
export const sweptRows = comparisonRows.filter((row) => row.haraWins === workloads.length);
export const maxLeadRow = haraLeadingRows.at(0) ?? null;
export const closestRow = comparisonRows.reduce<ComparisonRow | null>((best, row) => {
  if (!best) return row;
  return Math.abs(Math.log(row.overallRatio)) < Math.abs(Math.log(best.overallRatio)) ? row : best;
}, null);

const classIndex = new Map<string, number>(
  (classEvidence?.measurements ?? [])
    .filter((row) => row.status === "ok" && row.analysis?.steady_ns != null)
    .map((row) => [`${row.runtime}/${row.workload}`, row.analysis!.steady_ns!])
);
const classStatusIndex = new Map<string, string>(
  (classEvidence?.measurements ?? []).map((row) => [`${row.runtime}/${row.workload}`, row.status])
);
export const classStatusFor = (runtime: string, workload: string) =>
  classStatusIndex.get(`${runtime}/${workload}`) ?? "pending";
export const ratioForClass = (runtime: string, workload: string) => {
  const hara = classIndex.get(`${haraClassRuntime}/${workload}`);
  const candidate = classIndex.get(`${runtime}/${workload}`);
  return hara && candidate ? candidate / hara : null;
};
export const classTime = (runtime: string, workload: string) => classIndex.get(`${runtime}/${workload}`);
const buildClassRows = (runtimes: string[]): ComparisonRow[] =>
  runtimes
    .flatMap((runtime) => {
      const overallRatio = geometricMean(workloads.flatMap((workload) => {
        const ratio = ratioForClass(runtime, workload);
        return ratio == null ? [] : [ratio];
      }));
      if (overallRatio == null) return [];
      const haraWins = workloads.filter((workload) => (ratioForClass(runtime, workload) ?? 0) > 1).length;
      const competitorWins = workloads.filter((workload) => {
        const ratio = ratioForClass(runtime, workload);
        return ratio != null && ratio < 1;
      }).length;
      return [{
        runtime,
        label: runtimeLabels[runtime] ?? runtime.replace(/-prepared$/, ""),
        overallRatio,
        haraWins,
        competitorWins
      }];
    })
    .sort((left, right) => right.overallRatio - left.overallRatio);
export type ClassGroup = { id: string; title: string; summary: string; rows: ComparisonRow[] };
export const classGroups: ClassGroup[] = [
  {
    id: "class",
    title: "Best in class",
    summary: "Hara measured first against dynamic runtimes with adaptive compilation — its own class.",
    rows: buildClassRows(catalog.class_competitors.map((name) => `${name}-prepared`))
  },
  {
    id: "lisp",
    title: "Lisp family",
    summary: "Common Lisp, Scheme and Clojure-derived runtimes, grouped separately from the dynamic-JIT claim.",
    rows: buildClassRows(catalog.lisp_competitors.map((name) => `${name}-prepared`))
  },
  {
    id: "references",
    title: "Reference ceilings",
    summary: "C, Java and Python remain essential context as differently coloured reference ceilings, not peers.",
    rows: buildClassRows(catalog.reference_competitors.map((name) => `${name}-prepared`))
  }
].filter((group) => group.rows.length > 0);
const classTimestamp = classEvidence?.environment?.timestamp;
export const classEvidenceDate = classTimestamp && !Number.isNaN(Date.parse(classTimestamp))
  ? new Intl.DateTimeFormat("en-AU", {
      day: "numeric",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      timeZoneName: "short"
    }).format(new Date(classTimestamp))
  : "reference run pending";

export const comparisonResult = (ratio: number | null) => {
  if (ratio == null) return { value: "—", status: "pending", label: "pending", text: "Pending" };
  if (Math.abs(ratio - 1) < 0.015) {
    return { value: "1.00×", status: "parity", label: "parity", text: "Hara is effectively at parity" };
  }
  return ratio > 1
    ? { value: `${ratio.toFixed(2)}×`, status: "ahead", label: "ahead", text: `Hara is ${ratio.toFixed(2)}× faster` }
    : { value: `${(1 / ratio).toFixed(2)}×`, status: "behind", label: "behind", text: `Hara is ${(1 / ratio).toFixed(2)}× slower` };
};
export const formatTime = (ns: number | null | undefined) =>
  ns == null ? "Pending" : ns < 1e6 ? `${(ns / 1e3).toFixed(1)} µs` : `${(ns / 1e6).toFixed(2)} ms`;
export const formatRps = (value: number | null | undefined) =>
  value == null ? "Pending" : value >= 1e3 ? `${(value / 1e3).toFixed(1)}k` : value.toFixed(1);
export const languageTime = (runtime: string, workload: string) => languageIndex.get(`${runtime}/${workload}`);
export const httpRows = evidence?.http_measurements ?? [];
export const hasHttpEvidence = httpRows.length > 0;
export const httpRoutes = [...new Set(httpRows.map((row) => row.route))];
export const httpServers = [...new Set(httpRows.map((row) => row.server))];
export const httpIndex = new Map<string, HttpMeasurement>(
  httpRows.map((row) => [`${row.server}/${row.route}`, row])
);
export const httpBaseline = "hoplite-raw";
export const httpRatioFor = (server: string, route: string) => {
  const baseline = httpIndex.get(`${httpBaseline}/${route}`);
  const candidate = httpIndex.get(`${server}/${route}`);
  return baseline?.requests_per_second && candidate?.requests_per_second
    ? baseline.requests_per_second / candidate.requests_per_second
    : null;
};
export const runtimeBaseline = haraRuntime;
export const steadyRatioFor = (artifact: string, workload: string) => {
  const baseline = runtimeIndex.get(`${runtimeBaseline}/${workload}`);
  const candidate = runtimeIndex.get(`${artifact}/${workload}`);
  return baseline && candidate ? candidate.steady_ns / baseline.steady_ns : null;
};
const timestamp = languageEvidence?.environment?.timestamp ?? evidence?.environment?.timestamp;
export const evidenceDate = timestamp && !Number.isNaN(Date.parse(timestamp))
  ? new Intl.DateTimeFormat("en-AU", {
      day: "numeric",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
      timeZoneName: "short"
    }).format(new Date(timestamp))
  : "reference run pending";
