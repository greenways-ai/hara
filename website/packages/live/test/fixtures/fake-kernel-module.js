export let boots = 0;

export async function createDocsKernel(config) {
  boots += 1;
  return { config, boot: boots };
}
