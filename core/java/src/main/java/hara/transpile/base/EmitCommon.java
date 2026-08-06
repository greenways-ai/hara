package hara.transpile.base;

import hara.lang.data.Keyword;
import hara.lang.data.Symbol;
import hara.lang.base.G;
import java.util.*;

public class EmitCommon {

  public static class Context {
    public boolean explode = false;
    public boolean trace = false;
    public int indent = 0;
    public boolean compressed = false;
    public boolean multiline = false;
    public int maxLen = 60;
    public boolean emitInternal = false;
    public EmitFn emitFn = EmitHelper.defaultEmitFn;

    public Context copy() {
      Context c = new Context();
      c.explode = this.explode;
      c.trace = this.trace;
      c.indent = this.indent;
      c.compressed = this.compressed;
      c.multiline = this.multiline;
      c.maxLen = this.maxLen;
      c.emitInternal = this.emitInternal;
      c.emitFn = this.emitFn;
      return c;
    }
  }

  public static String newlineIndent(Context ctx) {
    if (!ctx.compressed) {
      StringBuilder sb = new StringBuilder("\n");
      for (int i = 0; i < ctx.indent; i++) {
        sb.append(" ");
      }
      return sb.toString();
    }
    return " ";
  }

  public static List<Object> formKey(Object form, Map grammar) {
    List<Object> baseKey = (List<Object>) EmitHelper.formKeyBase(form);

    // If it's an expression (list), check if the first element is reserved
    if (baseKey.get(0).equals(Keyword.create("expression"))) {
      if (form instanceof List && !((List) form).isEmpty()) {
        Object first = ((List) form).get(0);
        Map reservedMap =
            (Map) EmitHelper.getIn(grammar, Arrays.asList(Keyword.create("reserved")));
        if (reservedMap != null) {
          Map opProps = (Map) reservedMap.get(first);
          if (opProps != null) {
            Object op = opProps.get(Keyword.create("op"));
            Object type = opProps.get(Keyword.create("type"));
            if (type == null) type = Keyword.create("statement");
            return Arrays.asList(op, type, true);
          }
        }
        return Arrays.asList(Keyword.create("invoke"), Keyword.create("invoke"), false);
      }
    }

    return baseKey;
  }

  public static Object emitCommonLoop(Object form, Map grammar, Map mopts) {
    Context ctx = (Context) mopts.get("context");
    if (ctx == null) {
      ctx = new Context();
      ctx.emitFn = EmitCommon::emitCommonLoop;
      mopts.put("context", ctx);
    }

    List<Object> keyTuple = formKey(form, grammar);
    Object key = keyTuple.get(0);
    Object type = keyTuple.get(1);
    boolean check = keyTuple.size() > 2 ? (boolean) keyTuple.get(2) : false;

    if (check) {
      return emitOp(key, form, grammar, mopts);
    }

    // Default dispatch based on type
    if (type instanceof Keyword) {
      String tName = ((Keyword) type).getName();
      if (tName.equals("token")) return EmitHelper.emitToken(key, form, grammar, mopts);
      if (tName.equals("invoke")) return emitInvoke(null, (List) form, grammar, mopts);
      if (tName.equals("macro")) return emitMacro(key, form, grammar, mopts);
    }

    return EmitHelper.defaultEmitFn.apply(form, grammar, mopts);
  }

  public static Object emitOp(Object key, Object form, Map grammar, Map mopts) {
    Object sym = null;
    if (form instanceof List && !((List) form).isEmpty()) sym = ((List) form).get(0);
    else if (form instanceof Symbol) sym = form;
    else return EmitHelper.defaultEmitFn.apply(form, grammar, mopts);

    Map props = (Map) EmitHelper.getIn(grammar, Arrays.asList(Keyword.create("reserved"), sym));
    if (props == null) return EmitHelper.defaultEmitFn.apply(form, grammar, mopts);

    Object emit = props.get(Keyword.create("emit"));
    Object raw = props.get(Keyword.create("raw"));
    if (raw == null) raw = G.display(sym);

    List args =
        (form instanceof List)
            ? ((List) form).subList(1, ((List) form).size())
            : Collections.emptyList();

    String emitStr = emit.toString();
    if (emit instanceof Keyword) emitStr = ((Keyword) emit).getName();

    switch (emitStr) {
      case "discard":
        return "";
      case "free":
        return emitFree(props, (List) form, grammar, mopts);
      case "squash":
        return emitSquash(key, (List) form, grammar, mopts);
      case "token":
        return raw;
      case "unit":
        return emitUnit(props, (List) form, grammar, mopts);
      case "internal":
        return emitInternal((List) form, grammar, mopts);
      case "pre":
        return emitPre(raw, args, grammar, mopts);
      case "post":
        return emitPost(raw, args, grammar, mopts);
      case "prefix":
        return emitPrefix(raw, args, grammar, mopts);
      case "postfix":
        return emitPostfix(raw, args, grammar, mopts);
      case "infix":
        return emitInfix(raw, args, grammar, mopts);
      case "bi":
        return emitBi(raw, args, grammar, mopts);
      case "assign":
        return emitAssign(raw, args, grammar, mopts);
      case "invoke":
        return emitInvoke(raw, args, grammar, mopts);
      case "new":
        return emitNew(raw, args, grammar, mopts);
      case "static-invoke":
        return emitStaticInvoke(raw, args, grammar, mopts);
      case "index":
        return emitIndex(raw, args, grammar, mopts);
      case "return":
        return emitReturn(raw, args, grammar, mopts);
      case "macro":
        return emitMacro(key, form, grammar, mopts);
      case "block":
        return EmitBlock.emitBlock(key, form, grammar, mopts);
      case "fn":
        return EmitFnImpl.emitFn(key, form, grammar, mopts);
      default:
        throw new RuntimeException("No matching clause for emit: " + emitStr);
    }
  }

  // Helpers
  public static String emitFree(Object props, List form, Map grammar, Map mopts) {
    List args = form.subList(1, form.size());
    return String.join(" ", emitArray(args, grammar, mopts));
  }

  public static String emitSquash(Object key, List form, Map grammar, Map mopts) {
    List args = form.subList(1, form.size());
    return String.join("", emitArray(args, grammar, mopts));
  }

  public static String emitUnit(Object props, List form, Map grammar, Map mopts) {
    return "";
  }

  public static String emitInternal(List form, Map grammar, Map mopts) {
    Object value = form.get(1);
    Context ctx = (Context) mopts.get("context");
    return ctx.emitFn.apply(value, grammar, mopts).toString();
  }

  public static String emitPre(Object raw, List args, Map grammar, Map mopts) {
    if (args.size() != 1) throw new RuntimeException("Single argument for " + raw);
    Context ctx = (Context) mopts.get("context");
    return raw.toString() + ctx.emitFn.apply(args.get(0), grammar, mopts);
  }

  public static String emitPost(Object raw, List args, Map grammar, Map mopts) {
    if (args.size() != 1) throw new RuntimeException("Single argument for " + raw);
    Context ctx = (Context) mopts.get("context");
    return ctx.emitFn.apply(args.get(0), grammar, mopts) + raw.toString();
  }

  public static String emitPrefix(Object raw, List args, Map grammar, Map mopts) {
    if (args.size() != 1) throw new RuntimeException("Single argument for " + raw);
    Context ctx = (Context) mopts.get("context");
    return raw.toString() + " " + ctx.emitFn.apply(args.get(0), grammar, mopts);
  }

  public static String emitPostfix(Object raw, List args, Map grammar, Map mopts) {
    if (args.size() != 1) throw new RuntimeException("Single argument for " + raw);
    Context ctx = (Context) mopts.get("context");
    return ctx.emitFn.apply(args.get(0), grammar, mopts) + " " + raw.toString();
  }

  public static String emitInfix(Object raw, List args, Map grammar, Map mopts) {
    if (args.isEmpty()) throw new RuntimeException("One or more arguments for " + raw);
    String sep = " " + raw.toString() + " ";
    return String.join(sep, emitArray(args, grammar, mopts));
  }

  public static String emitBi(Object raw, List args, Map grammar, Map mopts) {
    if (args.size() != 2) throw new RuntimeException("Two arguments for " + raw);
    return emitInfix(raw, args, grammar, mopts);
  }

  public static String emitAssign(Object raw, List args, Map grammar, Map mopts) {
    if (args.size() != 2) throw new RuntimeException("Two arguments for assign");
    return String.join(" " + raw.toString() + " ", emitArray(args, grammar, mopts));
  }

  public static String emitInvoke(Object raw, List args, Map grammar, Map mopts) {
    String argsStr = String.join(", ", emitArray(args, grammar, mopts));
    return (raw != null ? raw.toString() : "") + "(" + argsStr + ")";
  }

  public static String emitNew(Object raw, List args, Map grammar, Map mopts) {
    Object sym = args.get(0);
    List more = args.subList(1, args.size());
    return raw.toString() + " " + emitInvoke(sym, more, grammar, mopts);
  }

  public static String emitStaticInvoke(Object raw, List args, Map grammar, Map mopts) {
    return emitInvoke(raw, args, grammar, mopts);
  }

  public static String emitIndex(Object raw, List args, Map grammar, Map mopts) {
    Object sym = args.get(0);
    List more = args.subList(1, args.size());
    Context ctx = (Context) mopts.get("context");
    StringBuilder sb = new StringBuilder();
    sb.append(ctx.emitFn.apply(sym, grammar, mopts));
    for (Object idx : more) {
      sb.append("[").append(ctx.emitFn.apply(idx, grammar, mopts)).append("]");
    }
    return sb.toString();
  }

  public static String emitReturn(Object raw, List args, Map grammar, Map mopts) {
    Context ctx = (Context) mopts.get("context");
    String val = "";
    if (!args.isEmpty()) {
      val = " " + ctx.emitFn.apply(args.get(0), grammar, mopts);
    }
    return raw.toString() + val;
  }

  public static Object emitMacro(Object key, Object form, Map grammar, Map mopts) {
    return "MACRO_NOT_IMPLEMENTED";
  }

  public static List<String> emitArray(List array, Map grammar, Map mopts) {
    Context ctx = (Context) mopts.get("context");
    List<String> res = new ArrayList<>();
    for (Object item : array) {
      res.add(ctx.emitFn.apply(item, grammar, mopts).toString());
    }
    return res;
  }
}
