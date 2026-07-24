# Complete Symbol List

## BuiltinBasic (basic)

### `atom`
Creates a standard atom with the given value.
```clojure
(atom ...)
```

### `atom:basic`
```clojure
(atom:basic ...)
```

### `compare`
```clojure
(compare ...)
```

### `counter`
```clojure
(counter ...)
```

### `deref`
Dereferences the given reference object.
```clojure
(deref ...)
```

### `flag`
```clojure
(flag ...)
```

### `hash`
```clojure
(hash ...)
```

### `keyword`
```clojure
(keyword ...)
```

### `meta`
```clojure
(meta ...)
```

### `realize`
```clojure
(realize ...)
```

### `realized?`
```clojure
(realized? ...)
```

### `reset!`
Sets the value of atom to new value without regard for the current value. Returns the new value.
```clojure
(reset! ...)
```

### `symbol`
```clojure
(symbol ...)
```

### `type`
```clojure
(type ...)
```

### `volatile`
```clojure
(volatile ...)
```

### `with-meta`
```clojure
(with-meta ...)
```

## BuiltinCheck (check)

### `boolean?`
```clojure
(boolean? ...)
```

### `false?`
```clojure
(false? ...)
```

### `nil?`
```clojure
(nil? ...)
```

### `true?`
```clojure
(true? ...)
```

### `zero?`
```clojure
(zero? ...)
```

## BuiltinRef (ref)

### `compare-and-set!`
```clojure
(compare-and-set! ...)
```

### `swap!`
```clojure
(swap! ...)
```

### `vreset!`
```clojure
(vreset! ...)
```

### `vswap!`
```clojure
(vswap! ...)
```

## BuiltinCollection (coll)

### `assoc`
```clojure
(assoc ...)
```

### `concat`
```clojure
(concat ...)
```

### `conj`
```clojure
(conj ...)
```

### `cons`
```clojure
(cons ...)
```

### `count`
```clojure
(count ...)
```

### `dissoc`
```clojure
(dissoc ...)
```

### `empty`
```clojure
(empty ...)
```

### `first`
```clojure
(first ...)
```

### `get`
```clojure
(get ...)
```

### `into`
```clojure
(into ...)
```

### `keys`
```clojure
(keys ...)
```

### `last`
```clojure
(last ...)
```

### `merge`
```clojure
(merge ...)
```

### `next`
```clojure
(next ...)
```

### `nth`
```clojure
(nth ...)
```

### `peek`
```clojure
(peek ...)
```

### `pop`
```clojure
(pop ...)
```

### `rest`
```clojure
(rest ...)
```

### `seq`
```clojure
(seq ...)
```

### `vals`
```clojure
(vals ...)
```

### `zipmap`
```clojure
(zipmap ...)
```

## BuiltinInterop (interop)

### `class`
```clojure
(class ...)
```

### `new`
```clojure
(new ...)
```

## BuiltinLambda (lambda)

### `F`
```clojure
(F ...)
```

### `T`
```clojure
(T ...)
```

### `apply`
```clojure
(apply ...)
```

### `call`
```clojure
(call ...)
```

### `comp`
```clojure
(comp ...)
```

### `group-by`
```clojure
(group-by ...)
```

### `identity`
```clojure
(identity ...)
```

### `juxt`
```clojure
(juxt ...)
```

### `keep`
```clojure
(keep ...)
```

### `map`
```clojure
(map ...)
```

### `map:apply`
```clojure
(map:apply ...)
```

### `map:entries`
```clojure
(map:entries ...)
```

### `map:juxt`
```clojure
(map:juxt ...)
```

### `map:keys`
```clojure
(map:keys ...)
```

### `map:vals`
```clojure
(map:vals ...)
```

### `mapcat`
```clojure
(mapcat ...)
```

### `partial`
```clojure
(partial ...)
```

### `partition:pair`
```clojure
(partition:pair ...)
```

### `pipe`
```clojure
(pipe ...)
```

### `reduce`
```clojure
(reduce ...)
```

### `reduce-in`
```clojure
(reduce-in ...)
```

## BuiltinOps (ops)

### `*`
```clojure
(* ...)
```

### `+`
```clojure
(+ ...)
```

### `-`
```clojure
(- ...)
```

### `/`
```clojure
(/ ...)
```

### `<`
```clojure
(< ...)
```

### `<=`
```clojure
(<= ...)
```

### `=`
```clojure
(= ...)
```

### `>`
```clojure
(> ...)
```

### `>=`
```clojure
(>= ...)
```

### `dec`
```clojure
(dec ...)
```

### `inc`
```clojure
(inc ...)
```

### `max`
```clojure
(max ...)
```

### `min`
```clojure
(min ...)
```

### `mod`
```clojure
(mod ...)
```

### `quot`
```clojure
(quot ...)
```

### `rem`
```clojure
(rem ...)
```

## BuiltinRuntime (rt)

### `eval`
```clojure
(eval ...)
```

### `load`
```clojure
(load ...)
```

### `sys:add-paths`
```clojure
(sys:add-paths ...)
```

### `sys:remove-paths`
```clojure
(sys:remove-paths ...)
```

## BuiltinStruct (struct)

### `hash-map`
```clojure
(hash-map ...)
```

### `hash-set`
```clojure
(hash-set ...)
```

### `list`
```clojure
(list ...)
```

### `vector`
```clojure
(vector ...)
```

## BuiltinTime (time)

### `now`
```clojure
(now ...)
```

## BuiltinNamespace (ns)

### `ns:aliases`
```clojure
(ns:aliases ...)
```

### `ns:create`
```clojure
(ns:create ...)
```

### `ns:find`
```clojure
(ns:find ...)
```

### `ns:imports`
```clojure
(ns:imports ...)
```

### `ns:list`
```clojure
(ns:list ...)
```

### `ns:map`
```clojure
(ns:map ...)
```

### `ns:name`
```clojure
(ns:name ...)
```

## BuiltinUtil (util)

### `pr-str`
```clojure
(pr-str ...)
```

### `println`
```clojure
(println ...)
```

## Macro (macro)

### `.`
```clojure
(. ...)
```

### `and`
```clojure
(and ...)
```

### `case`
```clojure
(case ...)
```

### `cond`
```clojure
(cond ...)
```

### `def`
```clojure
(def ...)
```

### `do`
```clojure
(do ...)
```

### `fn`
```clojure
(fn ...)
```

### `for`
```clojure
(for ...)
```

### `if`
```clojure
(if ...)
```

### `let`
```clojure
(let ...)
```

### `loop`
```clojure
(loop ...)
```

### `or`
```clojure
(or ...)
```

### `quote`
```clojure
(quote ...)
```

### `recur`
```clojure
(recur ...)
```

### `throw`
```clojure
(throw ...)
```

### `try`
```clojure
(try ...)
```

### `var`
```clojure
(var ...)
```
