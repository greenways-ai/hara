package hara.truffle;

import com.oracle.truffle.api.interop.TruffleObject;
import com.oracle.truffle.api.library.ExportLibrary;
import com.oracle.truffle.api.library.ExportMessage;
import com.oracle.truffle.api.interop.InteropLibrary;
import hara.lang.data.Symbol;
import hara.lang.data.Keyword;
import hara.lang.data.TaggedLiteral;
import hara.lang.protocol.IMetadata;
import hara.lang.protocol.ILookup;
import hara.lang.data.types.ILinearType;
import hara.truffle.bytecode.HbcProgram;
import hara.truffle.bytecode.HbcProgram.Function;
import hara.truffle.bytecode.HbcProgram.Instruction;
import hara.truffle.bytecode.HbcProgram.TryEntry;
import java.util.ArrayList;
import java.util.ArrayDeque;
import java.util.Arrays;

/** Executes a validated portable HBC3 program using the ordinary Hara runtime boundaries. */
public final class HbcMachine {
  private HbcMachine() {}

  public static Object execute(HbcProgram program, HaraContext context) {
    return HaraBox.export(call(program, context, program.entry(), new Object[0], new Object[0]));
  }

  private static Object call(
      HbcProgram program, HaraContext context, int functionIndex, Object[] arguments, Object[] captures) {
    Function function = program.functions().get(functionIndex);
    Object[] locals = bindLocals(function, arguments, captures);
    ArrayList<Object> stack = new ArrayList<>(function.maxStack());
    ArrayDeque<CallFrame> calls = new ArrayDeque<>();
    int ip = 0;
    while (true) {
      Instruction instruction = function.code().get(ip);
      try {
        switch (instruction.opcode()) {
        case CONSTANT -> stack.add(program.constants().get(index(instruction.first())));
        case NIL -> stack.add(null);
        case TRUE -> stack.add(true);
        case FALSE -> stack.add(false);
        case LOAD_LOCAL -> stack.add(locals[index(instruction.first())]);
        case STORE_LOCAL -> locals[index(instruction.first())] = pop(stack);
        case POP -> pop(stack);
        case DUP -> stack.add(peek(stack));
        case PRIMITIVE -> {
          Object[] args = popArguments(stack, index(instruction.second()));
          stack.add(invokePrimitive(context, index(instruction.first()), args));
        }
        case PRIMITIVE_LOCAL_CONST ->
            stack.add(
                invokePrimitive(
                    context,
                    index(instruction.first()),
                    new Object[] {
                      locals[index(instruction.second())],
                      program.constants().get(index(instruction.third()))
                    }));
        case JUMP -> {
          ip = index(instruction.first());
          continue;
        }
        case JUMP_IF_FALSE -> {
          if (!truthy(pop(stack))) {
            ip = index(instruction.first());
            continue;
          }
        }
        case CLOSURE -> {
          Object[] closed = popArguments(stack, index(instruction.second()));
          stack.add(new HbcClosure(program, context, index(instruction.first()), closed));
        }
        case CALL -> {
          Object[] args = popArguments(stack, index(instruction.first()));
          Object callee = HaraBox.unwrap(pop(stack));
          HbcClosure closure = selectClosure(callee, args.length);
          if (closure != null
              && closure.program == program
              && closure.context == context
              && closure.namespace.equals(context.currentNamespaceName())
              && !program.functions().get(closure.prototype).asyncFunction()) {
            calls.push(new CallFrame(functionIndex, function, locals, stack, ip + 1));
            functionIndex = closure.prototype;
            function = program.functions().get(functionIndex);
            locals = bindLocals(function, args, closure.captures);
            stack = new ArrayList<>(function.maxStack());
            ip = 0;
            continue;
          }
          stack.add(context.invokeCallable(callee, args));
        }
        case CALL_STATIC -> {
          Object[] args = popArguments(stack, index(instruction.second()));
          Function target = program.functions().get(index(instruction.first()));
          int currentCaptureBase = function.arity() + (function.variadic() ? 1 : 0);
          Object[] inherited =
              Arrays.copyOfRange(
                  locals,
                  currentCaptureBase,
                  Math.min(locals.length, currentCaptureBase + target.captureCount()));
          if (target.asyncFunction()) {
            int targetIndex = index(instruction.first());
            stack.add(context.hbcAsync(() -> call(program, context, targetIndex, args, inherited)));
          } else {
            calls.push(new CallFrame(functionIndex, function, locals, stack, ip + 1));
            functionIndex = index(instruction.first());
            function = target;
            locals = bindLocals(function, args, inherited);
            stack = new ArrayList<>(function.maxStack());
            ip = 0;
            continue;
          }
        }
        case GET_GLOBAL -> {
          String name = stringConstant(program, instruction.first());
          Integer primitive = primitiveId(name);
          stack.add(
              primitive == null
                  ? resolve(context, name).deref()
                  : new HbcNativeCallable(args -> invokePrimitive(context, primitive, args)));
        }
        case DEF_GLOBAL -> {
          Object value = pop(stack);
          Symbol symbol = Symbol.create(stringConstant(program, instruction.first()));
          IMetadata metadata = metadata(program, instruction.second());
          if (metadata != null) symbol = symbol.withMeta(metadata);
          context.define(symbol, value);
          stack.add(value);
        }
        case SET_GLOBAL -> {
          Object value = pop(stack);
          resolve(context, stringConstant(program, instruction.first())).reset(value);
          stack.add(value);
        }
        case VAR_GLOBAL -> stack.add(resolve(context, stringConstant(program, instruction.first())));
        case DECLARE_GLOBAL -> {
          context.declareCurrent(Symbol.create(stringConstant(program, instruction.first())));
          stack.add(null);
        }
        case DEF_STRUCT -> {
          String name = stringConstant(program, instruction.first());
          Object fieldsValue = program.constants().get(index(instruction.second()));
          if (!(fieldsValue instanceof ILinearType<?> fields)) {
            throw new HaraException("defstruct fields constant is not a vector");
          }
          String[] names = new String[Math.toIntExact(fields.count())];
          for (int i = 0; i < names.length; i++) {
            Object field = fields.nth(i);
            names[i] = field instanceof Symbol symbol ? symbol.getName() : String.valueOf(field);
          }
          HaraType type = new HaraType(name, names);
          context.define(Symbol.create(name), type);
          context.define(
              Symbol.create("->" + name),
              new HbcNativeCallable(
                  values -> {
                    if (values.length != names.length) {
                      throw new HaraException("constructor has no matching arity: " + values.length);
                    }
                    return new HaraStruct(type, values);
                  }));
          context.define(
              Symbol.create("map->" + name),
              new HbcNativeCallable(
                  values -> {
                    if (values.length != 1 || !(values[0] instanceof ILookup<?, ?> lookup)) {
                      throw new HaraException("map constructor expects one associative value");
                    }
                    Object[] members = new Object[names.length];
                    for (int i = 0; i < names.length; i++) {
                      members[i] = ((ILookup<Object, Object>) lookup).lookup(Keyword.create(names[i]));
                    }
                    return new HaraStruct(type, members);
                  }));
          stack.add(type);
        }
        case BUILD_VECTOR ->
            stack.add(hara.lang.data.Vector.Standard.from(null, popArguments(stack, index(instruction.first()))));
        case BUILD_LIST ->
            stack.add(hara.lang.data.List.Standard.from(null, popArguments(stack, index(instruction.first()))));
        case BUILD_MAP ->
            stack.add(hara.lang.data.Map.Standard.from(null, popArguments(stack, index(instruction.first()) * 2)));
        case BUILD_SET ->
            stack.add(hara.lang.data.Set.Standard.from(null, popArguments(stack, index(instruction.first()))));
        case CONCAT_LIST -> {
          Object[] values = popArguments(stack, index(instruction.first()));
          stack.add(invokeGlobal(context, "concat", values));
        }
        case TO_VECTOR -> stack.add(invokeGlobal(context, "vec", new Object[] {pop(stack)}));
        case MAKE_MULTI_ARITY -> {
          Object[] clauses = popArguments(stack, index(instruction.second()));
          HbcClosure[] closures = new HbcClosure[clauses.length];
          for (int i = 0; i < clauses.length; i++) {
            if (!(clauses[i] instanceof HbcClosure closure)) {
              throw new HaraException("multi-arity clauses must be functions");
            }
            closures[i] = closure;
          }
          stack.add(new HbcMultiArity(stringConstant(program, instruction.first()), closures));
        }
        case DEF_MACRO -> {
          Object value = pop(stack);
          Symbol symbol = Symbol.create(stringConstant(program, instruction.first()));
          IMetadata metadata = metadata(program, instruction.second());
          if (metadata != null) symbol = symbol.withMeta(metadata);
          context.defineMacro(
              symbol, new HaraMacro(context, context.currentNamespaceName(), symbol, value));
          stack.add(value);
        }
        case STRUCT_FIELD -> {
          Object target = pop(stack);
          if (!(target instanceof HaraStruct struct)) throw new HaraException("field expects a struct");
          try {
            stack.add(struct.read(stringConstant(program, instruction.first())));
          } catch (com.oracle.truffle.api.interop.UnknownIdentifierException error) {
            throw new HaraException("Unknown struct field: " + stringConstant(program, instruction.first()));
          }
        }
        case INSTANCE_OF -> {
          Object value = pop(stack);
          Object type = pop(stack);
          if (!(type instanceof HaraType)) throw new HaraException("instance? expects a struct type");
          stack.add(value instanceof HaraStruct struct && struct.type() == type);
        }
        case HOST_CALL -> {
          Object argumentsValue = pop(stack);
          Object method = pop(stack);
          Object service = pop(stack);
          stack.add(
              invokeGlobal(
                  context,
                  "std.native.Host/call",
                  new Object[] {service, method, argumentsValue}));
        }
        case AWAIT ->
            stack.add(
                invokeGlobal(
                    context, "std.foundation.coroutine/await", new Object[] {pop(stack)}));
        case RETURN -> {
          Object result = pop(stack);
          if (calls.isEmpty()) return result;
          CallFrame caller = calls.pop();
          functionIndex = caller.functionIndex;
          function = caller.function;
          locals = caller.locals;
          stack = caller.stack;
          stack.add(result);
          ip = caller.returnIp;
          continue;
        }
        case THROW -> throw new HbcThrown(pop(stack));
        case RETHROW -> throw new HbcThrown(pop(stack));
        }
      } catch (RuntimeException failure) {
        Integer target = routeFailure(function, ip, failure, locals, stack);
        while (target == null && !calls.isEmpty()) {
          CallFrame caller = calls.pop();
          functionIndex = caller.functionIndex;
          function = caller.function;
          locals = caller.locals;
          stack = caller.stack;
          target = routeFailure(function, caller.returnIp - 1, failure, locals, stack);
        }
        if (target == null) throw failure;
        ip = target;
        continue;
      }
      ip++;
    }
  }

  private static Object invokeGlobal(HaraContext context, String name, Object[] arguments) {
    return context.invokeCallable(resolve(context, name).deref(), arguments);
  }

  private static IMetadata metadata(HbcProgram program, long encodedIndex) {
    if (encodedIndex < 0) return null;
    java.util.List<HbcProgram.MetadataEntry> entries =
        program.varMetadata().get(index(encodedIndex));
    Object[] values = new Object[entries.size() * 2];
    for (int i = 0; i < entries.size(); i++) {
      values[i * 2] = metadataValue(entries.get(i).key());
      values[i * 2 + 1] = metadataValue(entries.get(i).value());
    }
    return hara.lang.data.Map.Standard.from(null, values);
  }

  @SuppressWarnings("unchecked")
  private static Object metadataValue(HbcProgram.MetadataValue metadata) {
    Object value = metadata.value();
    return switch (metadata.kind()) {
      case NIL, BOOLEAN, NUMBER, FLOAT, BIG_INTEGER, DECIMAL, REGEX, STRING, KEYWORD, SYMBOL -> value;
      case CHARACTER -> {
        int codePoint = ((Number) value).intValue();
        yield Character.isBmpCodePoint(codePoint)
            ? Character.valueOf((char) codePoint)
            : new String(Character.toChars(codePoint));
      }
      case TAGGED -> {
        HbcProgram.TaggedMetadata tagged = (HbcProgram.TaggedMetadata) value;
        yield new TaggedLiteral(Symbol.create(tagged.tag()), metadataValue(tagged.value()));
      }
      case VECTOR ->
          hara.lang.data.Vector.Standard.from(
              null,
              ((java.util.List<HbcProgram.MetadataValue>) value)
                  .stream().map(HbcMachine::metadataValue).toArray());
      case LIST ->
          hara.lang.data.List.Standard.from(
              null,
              ((java.util.List<HbcProgram.MetadataValue>) value)
                  .stream().map(HbcMachine::metadataValue).toArray());
      case SET ->
          hara.lang.data.Set.Standard.from(
              null,
              ((java.util.List<HbcProgram.MetadataValue>) value)
                  .stream().map(HbcMachine::metadataValue).toArray());
      case MAP -> {
        java.util.List<HbcProgram.MetadataEntry> entries =
            (java.util.List<HbcProgram.MetadataEntry>) value;
        Object[] pairs = new Object[entries.size() * 2];
        for (int i = 0; i < entries.size(); i++) {
          pairs[i * 2] = metadataValue(entries.get(i).key());
          pairs[i * 2 + 1] = metadataValue(entries.get(i).value());
        }
        yield hara.lang.data.Map.Standard.from(null, pairs);
      }
    };
  }

  private static Object[] bindLocals(Function function, Object[] arguments, Object[] captures) {
    checkArity(function, arguments.length);
    Object[] locals = new Object[function.localCount()];
    int fixed = function.arity();
    System.arraycopy(arguments, 0, locals, 0, Math.min(fixed, arguments.length));
    int captureBase = fixed;
    if (function.variadic()) {
      locals[fixed] =
          hara.lang.data.List.Standard.from(
              null, Arrays.copyOfRange(arguments, fixed, arguments.length));
      captureBase++;
    }
    System.arraycopy(captures, 0, locals, captureBase, captures.length);
    return locals;
  }

  private static HbcClosure selectClosure(Object callee, int arity) {
    if (callee instanceof HbcClosure closure) return closure;
    if (callee instanceof HbcMultiArity multi) {
      for (HbcClosure closure : multi.clauses) {
        Function function = closure.program.functions().get(closure.prototype);
        if ((!function.variadic() && function.arity() == arity)
            || (function.variadic() && arity >= function.arity())) return closure;
      }
    }
    return null;
  }

  private static Integer routeFailure(
      Function function,
      int errorIp,
      RuntimeException failure,
      Object[] locals,
      ArrayList<Object> stack) {
    for (int i = function.handlers().size() - 1; i >= 0; i--) {
      TryEntry handler = function.handlers().get(i);
      if (errorIp < handler.start() || errorIp >= handler.end()) continue;
      for (HbcProgram.CatchEntry clause : handler.catches()) {
        if (!catchMatches(failure, clause.className())) continue;
        truncate(stack, handler.depth());
        locals[clause.binding()] = caughtValue(failure);
        return index(clause.target());
      }
      if (handler.finallyTarget() != null) {
        truncate(stack, handler.depth());
        locals[handler.pendingValue()] = caughtValue(failure);
        locals[handler.pendingError()] = true;
        return index(handler.finallyTarget());
      }
    }
    return null;
  }

  private static boolean catchMatches(RuntimeException failure, String className) {
    if ("Exception".equals(className) || "Throwable".equals(className)) return true;
    if (failure instanceof HbcThrown thrown && thrown.value instanceof HaraStruct struct) {
      String type = struct.type().name();
      return type.equals(className) || type.endsWith("/" + className);
    }
    return false;
  }

  private static Object caughtValue(RuntimeException failure) {
    return failure instanceof HbcThrown thrown ? thrown.value : failure.getMessage();
  }

  private static void truncate(ArrayList<Object> stack, int depth) {
    while (stack.size() > depth) stack.remove(stack.size() - 1);
  }

  private static HaraVar resolve(HaraContext context, String name) {
    HaraVar variable = context.resolve(Symbol.create(name));
    if (variable == null) throw new HaraException("Unbound var: " + name);
    return variable;
  }

  private static String stringConstant(HbcProgram program, long operand) {
    Object value = program.constants().get(index(operand));
    if (!(value instanceof String string)) throw new HaraException("HBC3 name constant is not a string");
    return string;
  }

  private static Object[] popArguments(ArrayList<Object> stack, int count) {
    int start = stack.size() - count;
    if (start < 0) throw new HaraException("HBC3 stack underflow");
    Object[] values = new Object[count];
    for (int i = 0; i < count; i++) values[i] = stack.remove(start);
    return values;
  }

  private static Object pop(ArrayList<Object> stack) {
    if (stack.isEmpty()) throw new HaraException("HBC3 stack underflow");
    return stack.remove(stack.size() - 1);
  }

  private static Object peek(ArrayList<Object> stack) {
    if (stack.isEmpty()) throw new HaraException("HBC3 stack underflow");
    return stack.get(stack.size() - 1);
  }

  private static int index(long value) {
    return Math.toIntExact(value);
  }

  private static boolean truthy(Object value) {
    return value != null && !Boolean.FALSE.equals(value);
  }

  private static void checkArity(Function function, int actual) {
    if ((!function.variadic() && actual != function.arity())
        || (function.variadic() && actual < function.arity())) {
      throw new HaraException("function has no matching arity: " + actual);
    }
  }

  private static String primitiveName(int id) {
    return switch (HbcProgram.Primitive.fromId(id)) {
      case ADD -> "+";
      case SUBTRACT -> "-";
      case MULTIPLY -> "*";
      case DIVIDE -> "/";
      case REMAINDER -> "mod";
      case EQUAL -> "=";
      case LESS -> "<";
      case LESS_OR_EQUAL -> "<=";
      case GREATER -> ">";
      case GREATER_OR_EQUAL -> ">=";
      case COUNT -> "count";
      case GET -> "get";
      case META -> "meta";
      case NTH -> "nth";
      case ASSOC -> "assoc";
      case FIRST -> "first";
      case REST -> "rest";
      case SECOND -> "second";
      case TO_MUTABLE -> "to-mutable";
      case TO_PERSISTENT -> "to-persistent";
    };
  }

  private static Object invokePrimitive(HaraContext context, int id, Object[] arguments) {
    if (HbcProgram.Primitive.fromId(id) == HbcProgram.Primitive.EQUAL) {
      if (arguments.length < 2) throw new HaraException("= expects at least 2 arguments");
      Object first = HaraBox.unwrap(arguments[0]);
      for (int i = 1; i < arguments.length; i++) {
        Object value = HaraBox.unwrap(arguments[i]);
        if (first instanceof Number
            && value instanceof Number
            && (!first.getClass().equals(value.getClass()) || !first.equals(value))) return false;
        if (!(first instanceof Number && value instanceof Number)
            && !hara.lang.base.Eq.eq(first, value)) return false;
      }
      return true;
    }
    try {
      return invokeGlobal(context, primitiveName(id), arguments);
    } catch (RuntimeException failure) {
      HbcProgram.Primitive primitive = HbcProgram.Primitive.fromId(id);
      if ((primitive == HbcProgram.Primitive.DIVIDE
              || primitive == HbcProgram.Primitive.REMAINDER)
          && failure.getMessage() != null
          && failure.getMessage().toLowerCase(java.util.Locale.ROOT).contains("divide by zero")) {
        throw new HaraException("division by zero");
      }
      throw failure;
    }
  }

  private static Integer primitiveId(String name) {
    for (HbcProgram.Primitive primitive : HbcProgram.Primitive.values()) {
      if (primitiveName(primitive.id()).equals(name)
          || (primitive == HbcProgram.Primitive.REMAINDER && "%".equals(name))) {
        return primitive.id();
      }
    }
    return null;
  }

  @ExportLibrary(InteropLibrary.class)
  static final class HbcClosure implements TruffleObject {
    final HbcProgram program;
    final HaraContext context;
    final int prototype;
    final Object[] captures;
    final String namespace;

    HbcClosure(HbcProgram program, HaraContext context, int prototype, Object[] captures) {
      this.program = program;
      this.context = context;
      this.prototype = prototype;
      this.captures = captures;
      this.namespace = context.currentNamespaceName();
    }

    Object invoke(Object[] arguments) {
      Function function = program.functions().get(prototype);
      if (function.asyncFunction()) {
        return context.hbcAsync(
            () ->
                context.callInNamespace(
                    namespace, () -> call(program, context, prototype, arguments, captures)));
      }
      return context.callInNamespace(
          namespace, () -> call(program, context, prototype, arguments, captures));
    }

    @ExportMessage
    boolean isExecutable() {
      return true;
    }

    @ExportMessage
    Object execute(Object[] arguments) {
      return HaraBox.export(invoke(arguments));
    }

    @ExportMessage
    Object toDisplayString(boolean allowSideEffects) {
      return "<fn>";
    }

    @Override
    public String toString() {
      return "<fn>";
    }
  }

  @ExportLibrary(InteropLibrary.class)
  static final class HbcMultiArity implements TruffleObject {
    final String name;
    final HbcClosure[] clauses;

    HbcMultiArity(String name, HbcClosure[] clauses) {
      this.name = name;
      this.clauses = clauses;
    }

    Object invoke(Object[] arguments) {
      for (HbcClosure clause : clauses) {
        Function function = clause.program.functions().get(clause.prototype);
        if ((!function.variadic() && function.arity() == arguments.length)
            || (function.variadic() && arguments.length >= function.arity())) {
          return clause.invoke(arguments);
        }
      }
      throw new HaraException(name + " has no arity " + arguments.length);
    }

    @ExportMessage
    boolean isExecutable() {
      return true;
    }

    @ExportMessage
    Object execute(Object[] arguments) {
      return HaraBox.export(invoke(arguments));
    }

    @ExportMessage
    Object toDisplayString(boolean allowSideEffects) {
      return "<fn>";
    }

    @Override
    public String toString() {
      return "<fn>";
    }
  }

  @ExportLibrary(InteropLibrary.class)
  static final class HbcNativeCallable implements TruffleObject {
    final java.util.function.Function<Object[], Object> implementation;

    HbcNativeCallable(java.util.function.Function<Object[], Object> implementation) {
      this.implementation = implementation;
    }

    Object invoke(Object[] arguments) {
      return implementation.apply(arguments);
    }

    @ExportMessage
    boolean isExecutable() {
      return true;
    }

    @ExportMessage
    Object execute(Object[] arguments) {
      return HaraBox.export(invoke(arguments));
    }

    @ExportMessage
    Object toDisplayString(boolean allowSideEffects) {
      return "<fn>";
    }

    @Override
    public String toString() {
      return "<fn>";
    }
  }

  private static final class HbcThrown extends RuntimeException {
    final Object value;

    HbcThrown(Object value) {
      super("thrown: " + value);
      this.value = value;
    }
  }

  private record CallFrame(
      int functionIndex,
      Function function,
      Object[] locals,
      ArrayList<Object> stack,
      int returnIp) {}
}
