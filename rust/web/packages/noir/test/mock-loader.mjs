export const NOIR_VERSION = "test-noir";
export const BACKEND_ID = "test-backend";

export class MemoryArtifactCache {}

export class NoirBrowserLoader {
  async compile(program) {
    return {
      format: "hara/ledger.noir/v1",
      programKey: `program:${program.name}`,
      loaderId: "mock-loader",
      compilerVersion: program.noirVersion,
      backendVersion: program.backendVersion,
      circuit: { source: program.source }
    };
  }

  async prove(artifact, inputs) {
    return {
      format: "hara.noir.proof/v1",
      programKey: artifact.programKey,
      loaderId: artifact.loaderId,
      proof: new Uint8Array([1, 2, 3]),
      publicInputs: Object.values(inputs)
    };
  }

  async verify(artifact, proof) {
    return artifact.programKey === proof.programKey;
  }
}
