package hara.verify.noir;

import hara.lang.protocol.IDisplay;
import hara.verify.crypto.Hash256;
import hara.verify.json.JsonValue;
import hara.verify.json.StrictJson;

import java.util.Base64;

/** An opaque proof and its public inputs, tied to one compiled Noir program. */
public final class NoirProof implements IDisplay {
  private final Hash256 programKey;
  private final String proofBase64;
  private final String publicInputsJson;
  private final String loaderId;

  public NoirProof(
      Hash256 programKey, String proofBase64, String publicInputsJson, String loaderId) {
    if (programKey == null) throw new IllegalArgumentException("Program key is required.");
    if (proofBase64 == null || proofBase64.isBlank()) {
      throw new IllegalArgumentException("Proof bytes cannot be empty.");
    }
    try {
      if (Base64.getDecoder().decode(proofBase64).length == 0) {
        throw new IllegalArgumentException("Proof bytes cannot be empty.");
      }
    } catch (IllegalArgumentException e) {
      throw new IllegalArgumentException("Proof bytes must use canonical base64.", e);
    }
    if (!(StrictJson.parseValue(publicInputsJson) instanceof JsonValue.Array)) {
      throw new IllegalArgumentException("Public inputs must be a strict JSON array.");
    }
    if (loaderId == null || loaderId.isBlank()) {
      throw new IllegalArgumentException("Loader identity is required.");
    }
    this.programKey = programKey;
    this.proofBase64 = proofBase64;
    this.publicInputsJson = publicInputsJson;
    this.loaderId = loaderId;
  }

  public Hash256 programKey() {
    return programKey;
  }

  public String proofBase64() {
    return proofBase64;
  }

  public String publicInputsJson() {
    return publicInputsJson;
  }

  public String loaderId() {
    return loaderId;
  }

  @Override
  public String display() {
    return "#noir/proof {\"" + programKey.hex() + "\" \"" + loaderId + "\"}";
  }
}
