package hara.truffle;

import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertFalse;
import static org.junit.Assert.assertTrue;

import java.nio.file.Path;
import org.graalvm.polyglot.Context;
import org.graalvm.polyglot.io.IOAccess;
import org.junit.Test;

public class StdLedgerChainTest {
  private static Context newContext() {
    return Context.newBuilder(HaraLanguage.ID)
        .currentWorkingDirectory(Path.of(".").toAbsolutePath())
        .allowIO(IOAccess.ALL)
        .build();
  }

  @Test
  public void genesisBlockPreservesStateAndHasNoSource() {
    try (Context context = newContext()) {
      assertEquals(
          "[{:counter 0} 0 :std.ledger/genesis true 1]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (let [c (std.ledger.chain/create {:counter 0})"
                      + "        b (first (std.ledger.chain/blocks c))]"
                      + "    [(std.ledger.chain/state c)"
                      + "     (get b :index)"
                      + "     (get b :actor)"
                      + "     (nil? (get b :source))"
                      + "     (count (std.ledger.chain/blocks c))]))")
              .toString());
    }
  }

  @Test
  public void appendEvaluatesSourceAndUpdatesState() {
    try (Context context = newContext()) {
      assertEquals(
          "[2 1 :alice 1000 {:counter 1}]",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (let [c (std.ledger.chain/append (std.ledger.chain/create {:counter 0})"
                      + "                              :alice"
                      + "                              1000"
                      + "                              '(fn [state ctx] (assoc state :counter 1)))"
                      + "        h (std.ledger.chain/head c)]"
                      + "    [(count (std.ledger.chain/blocks c))"
                      + "     (get h :index)"
                      + "     (get h :actor)"
                      + "     (get h :timestamp)"
                      + "     (std.ledger.chain/state c)]))")
              .toString());
    }
  }

  @Test
  public void validAcceptsFreshChain() {
    try (Context context = newContext()) {
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (let [c (std.ledger.chain/append (std.ledger.chain/create {:counter 0})"
                      + "                              :alice"
                      + "                              1000"
                      + "                              '(fn [state ctx] (assoc state :counter 1)))]"
                      + "    (std.ledger.chain/valid? c)))")
              .asBoolean());
    }
  }

  @Test
  public void validRejectsTamperedState() {
    try (Context context = newContext()) {
      assertFalse(
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (let [c (std.ledger.chain/append (std.ledger.chain/create {:counter 0})"
                      + "                              :alice"
                      + "                              1000"
                      + "                              '(fn [state ctx] (assoc state :counter 1)))"
                      + "        blocks (std.ledger.chain/blocks c)"
                      + "        tampered (assoc c :blocks"
                      + "                        [(first blocks)"
                      + "                         (assoc (second blocks) :state {:counter 99})])]"
                      + "    (std.ledger.chain/valid? tampered)))")
              .asBoolean());
    }
  }

  @Test
  public void validRejectsTamperedSource() {
    try (Context context = newContext()) {
      assertFalse(
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (let [c (std.ledger.chain/append (std.ledger.chain/create {:counter 0})"
                      + "                              :alice"
                      + "                              1000"
                      + "                              '(fn [state ctx] (assoc state :counter 1)))"
                      + "        blocks (std.ledger.chain/blocks c)"
                      + "        tampered (assoc c :blocks"
                      + "                        [(first blocks)"
                      + "                         (assoc (second blocks)"
                      + "                                :source"
                      + "                                '(fn [state ctx] (assoc state :counter 99)))])]"
                      + "    (std.ledger.chain/valid? tampered)))")
              .asBoolean());
    }
  }

  @Test
  public void deterministicReplayProducesSameState() {
    try (Context context = newContext()) {
      assertTrue(
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (let [source '(fn [state ctx] (assoc state :counter (inc (:counter state))))"
                      + "        c1 (std.ledger.chain/append (std.ledger.chain/create {:counter 0}) :alice 1000 source)"
                      + "        c2 (std.ledger.chain/append (std.ledger.chain/create {:counter 0}) :alice 1000 source)]"
                      + "    (= (std.ledger.chain/state c1) (std.ledger.chain/state c2))))")
              .asBoolean());
    }
  }

  @Test
  public void txMacroExpandsToQuotedFunction() {
    try (Context context = newContext()) {
      assertEquals(
          "(quote (fn [state ctx] (assoc state :x 1)))",
          context
              .eval(
                  HaraLanguage.ID,
                  "(do (require 'std.ledger.chain)"
                      + "  (macroexpand '(std.ledger.chain/tx (assoc state :x 1))))")
              .toString());
    }
  }

  @Test
  public void demoProducesExpectedFinalState() {
    try (Context context = newContext()) {
      assertEquals(
          "{:state {:counter 1 :log [{:who :bob :when 1001}]} :block-count 3 :valid? true}",
          context
              .eval(
                  HaraLanguage.ID,
                  "(load-file \"lib/examples/ledger/chain_demo.hal\")")
              .asString());
    }
  }
}
