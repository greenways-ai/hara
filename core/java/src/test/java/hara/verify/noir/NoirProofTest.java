package hara.verify.noir;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertThrows;

import hara.verify.crypto.Hash256;
import java.util.Base64;
import org.junit.Test;

public class NoirProofTest {
  private static final Hash256 KEY =
      Hash256.parse("6db358723409593229a47cd5271d5073dd67f38c55bc0a643857afb2206e8c5f");

  @Test
  public void proofEnvelopePreservesOpaqueProofAndPublicInputs() {
    String bytes = Base64.getEncoder().encodeToString(new byte[] {1, 2, 3});
    NoirProof proof = new NoirProof(KEY, bytes, "[\"0x31\"]", "loader");

    assertEquals(KEY, proof.programKey());
    assertEquals(bytes, proof.proofBase64());
    assertEquals("[\"0x31\"]", proof.publicInputsJson());
  }

  @Test
  public void malformedEnvelopesFailClosed() {
    assertThrows(IllegalArgumentException.class, () -> new NoirProof(KEY, "%%%", "[]", "loader"));
    assertThrows(
        IllegalArgumentException.class,
        () -> new NoirProof(KEY, "AQ==", "{\"not\":\"an array\"}", "loader"));
  }
}
