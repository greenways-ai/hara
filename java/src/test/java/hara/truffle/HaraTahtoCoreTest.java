package hara.truffle;

import org.junit.runner.RunWith;

/** Executes the portable Tahto core, model, and runtime code.test suites. */
@RunWith(HaraJUnitRunner.class)
@HaraTestSource({
  "lib/test/tahto/core_test.hal",
  "lib/test/tahto/core/runtime_test.hal",
  "lib/test/tahto/model/target_models_test.hal"
})
public final class HaraTahtoCoreTest {}
