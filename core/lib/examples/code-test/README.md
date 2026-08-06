# `code.test` example

Run the sample with the Truffle runtime:

```bash
./hara run lib/examples/code-test/basic.hal
```

Each file declares its own namespace. The `fact` forms register test bodies and the final
`(code.test/run {:namespace "..."})` call executes only that namespace, returning structured
results.

`advanced.hal` demonstrates matchers, fixtures, and filtering. `tasks.hal` demonstrates `std.lib.task` and `deftask`.

The Java-backed libraries are discovered through the Hara library-provider SPI and installed lazily when their namespace is required.
