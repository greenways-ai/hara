package hara.truffle;

import hara.lang.base.G;
import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.data.types.ILinearType;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Map.Entry;

/** Compiler-facing normalization of portable HAL schema forms. */
public final class HalcSchema {
  private HalcSchema() {}

  public sealed interface Type
      permits Primitive,
          Reference,
          Union,
          VectorType,
          Tuple,
          MapType,
          FunctionType,
          EnumType,
          Extension,
          Unknown {}

  public record Primitive(String name) implements Type {}

  public record Reference(String name) implements Type {}

  public record Union(List<Type> types) implements Type {
    public Union {
      types = List.copyOf(types);
    }
  }

  public record VectorType(Type item) implements Type {}

  public record Tuple(List<Type> items) implements Type {
    public Tuple {
      items = List.copyOf(items);
    }
  }

  public record Field(Object name, Type type) {}

  public record MapType(List<Field> fields) implements Type {
    public MapType {
      fields = List.copyOf(fields);
    }
  }

  public record Function(List<Type> fixed, Type rest, Type output) {
    public Function {
      fixed = List.copyOf(fixed);
    }
  }

  public record FunctionType(List<Function> arities) implements Type {
    public FunctionType {
      arities = List.copyOf(arities);
    }
  }

  public record EnumType(List<Object> values) implements Type {
    public EnumType {
      values = List.copyOf(values);
    }
  }

  public record Extension(String head, List<Object> arguments) implements Type {
    public Extension {
      arguments = List.copyOf(arguments);
    }
  }

  public record Unknown(Object surface) implements Type {}

  public static Type normalize(Object schema) {
    if (schema instanceof Keyword keyword) return new Primitive(keyword.getName());
    if (schema instanceof hara.lang.data.List<?> reference
        && reference.count() == 2
        && reference.nth(0) instanceof Symbol operator
        && operator.getNamespace() == null
        && "var".equals(operator.getName())) {
      if (!(reference.nth(1) instanceof Symbol target)) {
        throw invalid("named schema reference must target a symbol");
      }
      if (target.getNamespace() == null) {
        throw invalid("named schema reference is not fully qualified: " + target.display());
      }
      return new Reference(target.display());
    }
    if (!(schema instanceof hara.lang.data.Vector<?> vector) || vector.count() == 0) {
      return new Unknown(schema);
    }
    if (!(vector.nth(0) instanceof Keyword head)) return new Unknown(schema);
    List<Object> arguments = values(vector, 1);
    return switch (head.getName()) {
      case "or" -> normalizeUnion(arguments);
      case "maybe" -> {
        requireCount(head.getName(), arguments, 1);
        yield normalizeUnion(List.of(arguments.get(0), Keyword.create("nil")));
      }
      case "vector" -> {
        requireCount(head.getName(), arguments, 1);
        yield new VectorType(normalize(arguments.get(0)));
      }
      case "tuple" -> new Tuple(normalizeAll(arguments));
      case "map" -> normalizeMap(arguments);
      case "fn" -> new FunctionType(List.of(normalizeFunction(vector)));
      case "function" -> {
        if (arguments.isEmpty()) {
          throw invalid(":function schema requires at least one :fn schema");
        }
        List<Function> arities = new ArrayList<>();
        for (Object argument : arguments) {
          if (!(argument instanceof hara.lang.data.Vector<?> function)) {
            throw invalid(":function members must be :fn schemas");
          }
          arities.add(normalizeFunction(function));
        }
        yield new FunctionType(arities);
      }
      case "enum" -> new EnumType(arguments);
      default -> new Extension(head.getName(), arguments);
    };
  }

  /** Conservative body-derived function facts used by lowering tiers. */
  public static Map<String, Type> inferFunctionTypes(
      String namespace,
      Object[] forms,
      Map<String, Type> declarations,
      Map<String, Type> definitions) {
    Map<String, Type> inferred = new HashMap<>();
    for (Object form : forms) {
      if (!(form instanceof hara.lang.data.List<?> definition) || definition.count() < 3) continue;
      if (!(definition.nth(0) instanceof Symbol operator)
          || operator.getNamespace() != null
          || !("defn".equals(operator.getName()) || "defn-".equals(operator.getName()))) continue;
      if (!(definition.nth(1) instanceof Symbol name)) continue;
      int parametersAt = -1;
      for (int index = 2; index < definition.count(); index++) {
        if (definition.nth(index) instanceof hara.lang.data.Vector<?>) {
          parametersAt = index;
          break;
        }
      }
      String qualified = namespace + "/" + name.getName();
      if (parametersAt < 0) {
        List<Function> arities = new ArrayList<>();
        for (int index = 2; index < definition.count(); index++) {
          if (!(definition.nth(index) instanceof hara.lang.data.List<?> clause)
              || clause.count() < 2
              || !(clause.nth(0) instanceof hara.lang.data.Vector<?>)) continue;
          Object[] single = new Object[Math.toIntExact(clause.count()) + 2];
          single[0] = definition.nth(0);
          single[1] = definition.nth(1);
          for (int item = 0; item < clause.count(); item++) single[item + 2] = clause.nth(item);
          hara.lang.data.List<?> synthetic = hara.lang.data.List.Standard.from(null, single);
          Type type =
              inferFunctionTypes(
                      namespace, new Object[] {synthetic}, declarations, definitions)
                  .get(qualified);
          if (type instanceof FunctionType functions) arities.addAll(functions.arities());
        }
        if (!arities.isEmpty()) inferred.put(qualified, new FunctionType(arities));
        continue;
      }
      hara.lang.data.Vector<?> parameters =
          (hara.lang.data.Vector<?>) definition.nth(parametersAt);
      Function declared = matchingArity(
          resolve(declarations.get(qualified), definitions), parameters);
      Map<String, Type> environment = new HashMap<>();
      List<Type> fixed = new ArrayList<>();
      Type rest = null;
      int fixedIndex = 0;
      boolean variadic = false;
      for (int index = 0; index < parameters.count(); index++) {
        Object parameter = parameters.nth(index);
        if (parameter instanceof Symbol marker
            && marker.getNamespace() == null
            && "&".equals(marker.getName())) {
          variadic = true;
          continue;
        }
        if (!(parameter instanceof Symbol parameterName)) continue;
        Type parameterType = variadic
            ? declared != null && declared.rest() != null ? declared.rest() : unknown()
            : declared != null && fixedIndex < declared.fixed().size()
                ? declared.fixed().get(fixedIndex)
                : unknown();
        environment.put(parameterName.getName(), parameterType);
        if (variadic) rest = parameterType;
        else {
          fixed.add(parameterType);
          fixedIndex++;
        }
      }
      Type output = new Primitive("nil");
      for (int index = parametersAt + 1; index < definition.count(); index++) {
        output = inferExpression(definition.nth(index), environment);
      }
      inferred.put(qualified, new FunctionType(List.of(new Function(fixed, rest, output))));
    }
    return Map.copyOf(inferred);
  }

  private static Type resolve(Type type, Map<String, Type> definitions) {
    HashSet<String> visited = new HashSet<>();
    while (type instanceof Reference reference && visited.add(reference.name())) {
      Type next = definitions.get(reference.name());
      if (next == null) break;
      type = next;
    }
    return type;
  }

  private static Function matchingArity(Type type, ILinearType<?> parameters) {
    if (!(type instanceof FunctionType functions)) return null;
    int fixed = 0;
    boolean variadic = false;
    for (int index = 0; index < parameters.count(); index++) {
      Object parameter = parameters.nth(index);
      if (parameter instanceof Symbol marker && "&".equals(marker.getName())) variadic = true;
      else if (!variadic) fixed++;
    }
    for (Function function : functions.arities()) {
      if (function.fixed().size() == fixed && (function.rest() != null) == variadic) return function;
    }
    return null;
  }

  private static Type inferExpression(Object form, Map<String, Type> environment) {
    if (form == null) return new Primitive("nil");
    if (form instanceof Boolean) return new Primitive("bool");
    if (form instanceof Byte || form instanceof Short || form instanceof Integer || form instanceof Long)
      return new Primitive("int");
    if (form instanceof Float || form instanceof Double) return new Primitive("float");
    if (form instanceof java.math.BigInteger) return new Primitive("bigint");
    if (form instanceof java.math.BigDecimal) return new Primitive("decimal");
    if (form instanceof Character) return new Primitive("char");
    if (form instanceof java.util.regex.Pattern) return new Primitive("regex");
    if (form instanceof String) return new Primitive("str");
    if (form instanceof Keyword) return new Primitive("keyword");
    if (form instanceof Symbol symbol)
      return environment.getOrDefault(symbol.getName(), unknown());
    if (form instanceof hara.lang.data.Vector<?> vector) {
      List<Type> members = new ArrayList<>();
      for (int index = 0; index < vector.count(); index++)
        pushJoined(members, inferExpression(vector.nth(index), environment));
      return new VectorType(join(members));
    }
    if (form instanceof hara.lang.data.types.IMapType<?, ?> map) {
      List<Field> fields = new ArrayList<>();
      for (Object item : map) {
        Entry<?, ?> entry = (Entry<?, ?>) item;
        fields.add(new Field(entry.getKey(), inferExpression(entry.getValue(), environment)));
      }
      return new MapType(fields);
    }
    if (form instanceof hara.lang.data.types.ISetType<?>)
      return new Extension("set", List.of());
    if (!(form instanceof hara.lang.data.List<?> list) || list.count() == 0
        || !(list.nth(0) instanceof Symbol operator)) return unknown();
    return inferList(list, operator.getName(), environment);
  }

  private static Type inferList(
      hara.lang.data.List<?> list, String operator, Map<String, Type> environment) {
    switch (operator) {
      case "do": {
        Type output = new Primitive("nil");
        for (int index = 1; index < list.count(); index++)
          output = inferExpression(list.nth(index), environment);
        return output;
      }
      case "if": {
        List<Type> branches = new ArrayList<>();
        for (int index = 2; index < list.count(); index++)
          pushJoined(branches, inferExpression(list.nth(index), environment));
        return join(branches);
      }
      case "let": {
        Map<String, Type> nested = new HashMap<>(environment);
        if (list.count() > 1 && list.nth(1) instanceof hara.lang.data.Vector<?> bindings) {
          for (int index = 0; index + 1 < bindings.count(); index += 2) {
            if (bindings.nth(index) instanceof Symbol name)
              nested.put(name.getName(), inferExpression(bindings.nth(index + 1), nested));
          }
        }
        Type output = new Primitive("nil");
        for (int index = 2; index < list.count(); index++)
          output = inferExpression(list.nth(index), nested);
        return output;
      }
      case "+", "-", "*", "%", "mod": {
        List<Type> operands = new ArrayList<>();
        for (int index = 1; index < list.count(); index++)
          pushJoined(operands, inferExpression(list.nth(index), environment));
        Type joined = join(operands);
        if (joined instanceof Primitive primitive
            && List.of("int", "float", "bigint", "decimal").contains(primitive.name())) return joined;
        return new Primitive("number");
      }
      case "/": return new Primitive("number");
      case "=", "<", "<=", ">", ">=", "instance?": return new Primitive("bool");
      case "count": return new Primitive("int");
      default: return unknown();
    }
  }

  private static Unknown unknown() {
    return new Unknown(Symbol.create("?"));
  }

  private static Type join(List<Type> members) {
    if (members.isEmpty()) return unknown();
    return members.size() == 1 ? members.get(0) : new Union(members);
  }

  private static void pushJoined(List<Type> output, Type type) {
    if (type instanceof Union union) {
      for (Type member : union.types()) pushUnique(output, member);
    } else pushUnique(output, type);
  }

  /** Canonical reader-form bridge used by the cross-runtime HBC schema codec. */
  public static Object readSurface(String source) {
    Object[] forms = HaraLanguage.readAll(source, "hbc:schema");
    if (forms.length != 1) throw invalid("schema surface must contain one form");
    return forms[0];
  }

  /** Canonical readable spelling used by the cross-runtime HBC schema codec. */
  public static String displaySurface(Object value) {
    return G.display(value);
  }

  private static Type normalizeUnion(List<Object> arguments) {
    if (arguments.isEmpty()) throw invalid(":or schema requires at least one member");
    List<Type> members = new ArrayList<>();
    for (Object argument : arguments) {
      Type normalized = normalize(argument);
      if (normalized instanceof Union union) {
        for (Type member : union.types()) pushUnique(members, member);
      } else {
        pushUnique(members, normalized);
      }
    }
    return members.size() == 1 ? members.get(0) : new Union(members);
  }

  private static Type normalizeMap(List<Object> arguments) {
    List<Field> fields = new ArrayList<>();
    for (Object argument : arguments) {
      if (!(argument instanceof hara.lang.data.Vector<?> pair) || pair.count() != 2) {
        throw invalid(":map schema fields must be [name type] pairs");
      }
      fields.add(new Field(pair.nth(0), normalize(pair.nth(1))));
    }
    return new MapType(fields);
  }

  private static Function normalizeFunction(ILinearType<?> function) {
    if (function.count() != 3
        || !(function.nth(0) instanceof Keyword head)
        || !"fn".equals(head.getName())) {
      throw invalid(":fn schema must be [:fn [inputs ...] output]");
    }
    if (!(function.nth(1) instanceof hara.lang.data.Vector<?> inputs)) {
      throw invalid(":fn schema inputs must be a vector");
    }
    List<Type> fixed = new ArrayList<>();
    Type rest = null;
    int index = 0;
    while (index < inputs.count()) {
      if (inputs.nth(index) instanceof Symbol marker
          && marker.getNamespace() == null
          && "&".equals(marker.getName())) {
        if (rest != null || index + 2 != inputs.count()) {
          throw invalid(":fn schema & must precede exactly one rest type");
        }
        rest = normalize(inputs.nth(index + 1));
        index += 2;
      } else {
        fixed.add(normalize(inputs.nth(index++)));
      }
    }
    return new Function(fixed, rest, normalize(function.nth(2)));
  }

  private static List<Type> normalizeAll(List<Object> values) {
    List<Type> output = new ArrayList<>(values.size());
    for (Object value : values) output.add(normalize(value));
    return output;
  }

  private static List<Object> values(ILinearType<?> input, int start) {
    List<Object> output = new ArrayList<>(Math.toIntExact(input.count()) - start);
    for (int index = start; index < input.count(); index++) output.add(input.nth(index));
    return output;
  }

  private static void requireCount(String head, List<Object> arguments, int expected) {
    if (arguments.size() != expected) {
      throw invalid(
          ":"
              + head
              + " schema expects "
              + expected
              + (expected == 1 ? " argument, got " : " arguments, got ")
              + arguments.size());
    }
  }

  private static void pushUnique(List<Type> output, Type value) {
    if (!output.contains(value)) output.add(value);
  }

  private static HaraException invalid(String detail) {
    return new HaraException(detail);
  }
}
