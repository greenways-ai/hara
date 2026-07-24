# JVM native flavor

The JVM provider is implemented internally under `hara.kernel.jvm` and selected per namespace:

```clojure
(ns example.jvm
  (:flavor :jvm)
  (:import [java.lang String RuntimeException]
           [java.awt Point]))
```

`:import` is a true namespace import. Only imported simple class names resolve, and qualified
symbols such as `String/valueOf` resolve static fields or callable static methods. Construction uses
`(new String "value")`.

The dot form traverses left to right. A symbol reads a field, a list invokes a method, and a
one-element vector performs the JVM extension for indexed access:

```clojure
(. point x)
(. point x (toString))
(. value child parent)
(. values [0])
```

Persistent Hara values retain portable lookup and protocol semantics; they do not fall through to
reflection. JVM reflection/invocation requires the reflection capability. Classpath and compiler
operations require their distinct grants. Imported JVM throwable classes participate in `catch`;
reflection wrappers are unwrapped and the original Java cause is preserved.

The public provider namespace family is `hara.native.jvm`, `hara.native.jvm.reflect`,
`hara.native.jvm.classpath`, and `hara.native.jvm.compiler`. Provider selection itself does not
implicitly refer these APIs into the current namespace.
