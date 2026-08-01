package hara.truffle;

import org.junit.runner.RunWith;

/** Executes the protocol-backed Tahto compiler and registry code.test suites. */
@RunWith(HaraJUnitRunner.class)
@HaraTestSource({
  "lib/test/tahto/protocol_test.hal",
  "lib/test/tahto/common/compiler_test.hal",
  "lib/test/tahto/common/grammar_api_test.hal",
  "lib/test/tahto/common/emit_test.hal"
})
public final class HaraTahtoCompilerTest {}
