package hara.verify.noir;

import hara.lang.protocol.IDisplay;
import hara.verify.crypto.Hash256;

/** A compiled ACIR artifact tied to the exact program identity that produced it. */
public final class NoirArtifact implements IDisplay {
  private final Hash256 programKey;
  private final String circuitJson;
  private final String loaderId;

  public NoirArtifact(Hash256 programKey, String circuitJson, String loaderId) {
    if (programKey == null) throw new IllegalArgumentException("Program key is required.");
    if (circuitJson == null || circuitJson.isBlank()) {
      throw new IllegalArgumentException("Compiled circuit JSON cannot be empty.");
    }
    if (loaderId == null || loaderId.isBlank()) {
      throw new IllegalArgumentException("Loader identity is required.");
    }
    this.programKey = programKey;
    this.circuitJson = circuitJson;
    this.loaderId = loaderId;
  }

  public Hash256 programKey() {
    return programKey;
  }

  public String circuitJson() {
    return circuitJson;
  }

  public String loaderId() {
    return loaderId;
  }

  @Override
  public String display() {
    return "#noir/artifact {\"" + programKey.hex() + "\" \"" + loaderId + "\"}";
  }
}
