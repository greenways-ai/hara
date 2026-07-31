let providerPromise;

export function createNoirProvider(loaderUrl) {
  providerPromise ??= import(loaderUrl).then(module => {
    const loader = new module.NoirBrowserLoader({ cache: new module.MemoryArtifactCache() });
    return { module, loader };
  });
  return providerPromise;
}

export async function callNoir(loaderUrl, operation, rawArguments) {
  const { module, loader } = await createNoirProvider(loaderUrl);
  const args = rawArguments.map(fromHta);
  if (operation === "compile") {
    const input = args[0] ?? {};
    const program = {
      name: required(input, "name"),
      source: required(input, "source"),
      noirVersion: input.noirVersion ?? input["noir-version"] ?? module.NOIR_VERSION,
      backendVersion: input.backendVersion ?? input["backend-version"] ?? module.BACKEND_ID
    };
    const artifact = await loader.compile(program);
    return artifactEnvelope(artifact);
  }
  if (operation === "prove") {
    const artifact = restoreArtifact(args[0]);
    const proof = await loader.prove(artifact, args[1]);
    return proofEnvelope(proof);
  }
  if (operation === "verify") {
    return loader.verify(restoreArtifact(args[0]), restoreProof(args[1]));
  }
  throw new Error(`noir/operation-unknown: ${operation}`);
}

function artifactEnvelope(artifact) {
  return {
    format: artifact.format,
    programKey: artifact.programKey,
    loaderId: artifact.loaderId,
    compilerVersion: artifact.compilerVersion,
    backendVersion: artifact.backendVersion,
    circuitJson: JSON.stringify(artifact.circuit)
  };
}

function proofEnvelope(proof) {
  return {
    format: proof.format,
    programKey: proof.programKey,
    loaderId: proof.loaderId,
    proof: proof.proof,
    publicInputs: [...proof.publicInputs]
  };
}

function restoreArtifact(value) {
  if (!value || value.format !== "hara/ledger.noir/v1") {
    throw new TypeError("noir/artifact-format: expected hara/ledger.noir/v1");
  }
  return {
    format: value.format,
    programKey: value.programKey,
    loaderId: value.loaderId,
    compilerVersion: value.compilerVersion,
    backendVersion: value.backendVersion,
    circuit: JSON.parse(required(value, "circuitJson"))
  };
}

function restoreProof(value) {
  if (!value || value.format !== "hara.noir.proof/v1") {
    throw new TypeError("noir/proof-format: expected hara.noir.proof/v1");
  }
  return {
    format: value.format,
    programKey: value.programKey,
    loaderId: value.loaderId,
    proof: value.proof,
    publicInputs: value.publicInputs
  };
}

function required(value, name) {
  const result = value?.[name];
  if (typeof result !== "string" || result.length === 0) {
    throw new TypeError(`noir/${name} must be a non-empty string`);
  }
  return result;
}

function fromHta(value) {
  if (value instanceof Map) {
    const result = {};
    for (const [key, item] of value) {
      const name = typeof key === "string" ? key : key?.name;
      if (typeof name !== "string") throw new TypeError("noir/map keys must be strings or keywords");
      result[toCamel(name)] = fromHta(item);
    }
    return result;
  }
  if (Array.isArray(value)) return value.map(fromHta);
  if (value instanceof Set) return [...value].map(fromHta);
  return value;
}

function toCamel(value) {
  return value.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());
}
