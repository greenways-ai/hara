# Task 2 Report: Remove redundant happy-path methods from Java tests

## Summary

Removed the happy-path Java test methods specified in `task-2-brief.md`. All error-asserting and JVM-specific tests were kept untouched. Unused imports were removed only where they became genuinely unused after deletion (`assertEquals` and `assertFalse` in `StdFoundationCoroutineTest.java`).

## Files changed and methods removed

### `java/src/test/java/hara/truffle/StdFoundationTest.java`

Removed 5 methods:
- `halFoundationOwnsMapAndFillsPortableSymbols`
- `mapIsEagerDirectAndLazyWhenCurried`
- `collectionAndOrderingAdditionsArePortableHaraValues`
- `atomsValidateMutateAndNotifyFourArgumentWatches`
- `foundationMacrosReceivePortableFormAndEnvironment`

Kept:
- `fallbackReloadRefreshesHalFoundation`
- `optimizedOperationsMatchTheirHalDefinitions`

No imports changed (kept tests still use `InputStream`, `StandardCharsets`, `Context`, `assertEquals`, `assertTrue`).

### `java/src/test/java/hara/truffle/StdFoundationCoroutineTest.java`

Removed 9 methods:
- `createMakesSuspendedCoroutine`
- `resumeRunsBodyToCompletion`
- `yieldExchangesValuesBothWays`
- `multiArgResumeDeliversVectorToYield`
- `multiYieldPacksVectorAndZeroYieldsNil`
- `yieldWorksFromNestedHelper`
- `nestedCoroutinesResumeEachOther`
- `generatorPipelineProducesLazily`
- `awaitReturnsSettledPromiseValue`

Kept all `PolyglotException`-asserting tests:
- `resumeOnDeadThrows`
- `bodyErrorRethrowsAtResumeAndKillsCoroutine`
- `closeOnNeverResumedCoroutine`
- `yieldOutsideCoroutineThrows`
- `reentrantResumeThrows`
- `closeRunsFinallyAndKillsCoroutine`
- `closeOnDeadIsNoOpAndCloseOnRunningThrows`
- `awaitRethrowsPromiseRejection`
- `awaitRejectsNonDerefable`

Imports removed because they became unused:
- `import static org.junit.Assert.assertEquals;`
- `import static org.junit.Assert.assertFalse;`

### `java/src/test/java/hara/truffle/HaraGeneratedLibrariesTest.java`

Removed 7 methods:
- `generatedLibrariesHaveDefaultAliasesWithoutLoadingFiles`
- `intrinsicsAllExplicitlyKeepsEveryDefaultAlias`
- `promisesRunAndAdoptCallbackPromises`
- `promisesExposePortableStateValueAndCancellation`
- `stringLibraryMatchesTheXtalkSurface`
- `strIsVariadicAndMatchesJvmConcatenation`
- `prStrUsesReadableHaraNotation`

Kept all `assertErrorContains`/`assertThrows` tests and the following listed methods:
- `generatedLibrariesAlsoSupportRequireAsAndRefer`
- `foundationNamespaceCombinesJavaAndHalSymbols`
- `intrinsicsCanExcludeAndRenameGeneratedAliases`
- `intrinsicsRejectUnknownConflictingAndDuplicateConfiguration`
- `completionIncludesGeneratedAliasesAndMarkerMethods`
- `dotCallsAreRestrictedToMarkedArraysAndObjects`
- `bitOperationsUseSignedThirtyTwoBitSemantics`
- `nestedLookupDoesNotConsumeItsPath`
- `emitterTypePredicatesAreAvailable`

No imports changed.

### `java/src/test/java/hara/truffle/HaraLanguageTest.java`

Removed 1 method:
- `defstructValuesBehaveLikePersistentMaps`

All other tests kept unchanged. No imports changed.

## Verification commands and output

### Maven Java tests

Command:
```shell
mvn -q -f java/pom.xml -Ptruffle test
```

Result: **BUILD SUCCESS** (exit code 0). Full output was large (Truffle interpreter-only warnings and test logs); the run completed without test failures.

### Hara library tests

Command:
```shell
./scripts/run-lib-tests
```

Output:
```
PASS  lib/test/std/foundation/alias_test.hal (4 checks)
PASS  lib/test/std/foundation/bytes_test.hal (1 checks)
PASS  lib/test/std/foundation/coroutine_test.hal (9 checks)
PASS  lib/test/std/foundation/promise_test.hal (10 checks)
PASS  lib/test/std/foundation/string_test.hal (27 checks)
PASS  lib/test/std/foundation_test.hal (13 checks)
PASS  lib/test/std/ledger/chain_test.hal (11 checks)
PASS  lib/test/std/struct_test.hal (9 checks)

lib/test summary: 8 passed, 0 failed, 8 total
```

Result: **All 8 test files passed.**

## Concerns or blockers

None. Both required verification commands passed. No logic or assertions in kept tests were modified. No git mutations were performed.
