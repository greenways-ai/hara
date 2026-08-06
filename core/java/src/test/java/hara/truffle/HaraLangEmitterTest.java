package hara.truffle;

import org.junit.runner.RunWith;

/** Executes every Lang common emitter code.test namespace in an isolated Hara context. */
@RunWith(HaraJUnitRunner.class)
@HaraTestSource({
  "lib/test/lang/common/emit_assign_test.hal",
  "lib/test/lang/common/emit_block_test.hal",
  "lib/test/lang/common/emit_common_test.hal",
  "lib/test/lang/common/emit_compat_test.hal",
  "lib/test/lang/common/emit_data_test.hal",
  "lib/test/lang/common/emit_fn_test.hal",
  "lib/test/lang/common/emit_helper_test.hal",
  "lib/test/lang/common/emit_orchestration_test.hal",
  "lib/test/lang/common/emit_preprocess_test.hal",
  "lib/test/lang/common/emit_rewrite_test.hal",
  "lib/test/lang/common/emit_special_test.hal",
  "lib/test/lang/common/emit_template_test.hal",
  "lib/test/lang/common/emit_test.hal",
  "lib/test/lang/common/emit_top_level_test.hal"
})
public final class HaraLangEmitterTest {}
