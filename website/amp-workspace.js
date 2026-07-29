const PRESETS = new Set(["flat", "hara", "bass", "voice"]);
const MODES = new Set(["spectrum", "scope", "artwork"]);

export function seedAmpWorkspace({
  project,
  workspace,
  visualizer,
  preset = "hara",
  mode = "spectrum"
}) {
  if (![project, workspace, visualizer].every((value) => typeof value === "string" && value.trim())) {
    throw new Error("Hara Amp workspace assets are incomplete");
  }
  const safePreset = PRESETS.has(preset) ? preset : "hara";
  const safeMode = MODES.has(mode) ? mode : "spectrum";
  const marker = ":recovery/journal true";
  if (!workspace.includes(marker)) throw new Error("Hara Amp workspace customizations are missing");
  const customized = workspace.replace(
    marker,
    `${marker}\n  :amp/eq-preset :${safePreset}\n  :amp/visual-mode :${safeMode}`
  );
  return new Map([
    ["/project.edn", project],
    ["/workspace.edn", customized],
    ["/src/visualizer.hal", visualizer]
  ]);
}
