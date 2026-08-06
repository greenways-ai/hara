package hara.truffle;

import org.junit.runner.RunWith;

/** Executes the portable Lang core, model, and runtime code.test suites. */
@RunWith(HaraJUnitRunner.class)
@HaraTestSource({
  "lib/test/lang/core_test.hal",
  "lib/test/lang/core/runtime_test.hal",
  "lib/test/lang/model/target_models_test.hal"
})
public final class HaraLangCoreTest {}
