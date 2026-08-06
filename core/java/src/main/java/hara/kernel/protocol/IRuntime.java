package hara.kernel.protocol;

import hara.lang.protocol.IColl;
import hara.lang.protocol.IContext;
import hara.lang.protocol.IFn;

import java.net.URL;
import java.util.Map.Entry;

@SuppressWarnings("rawtypes")
public interface IRuntime<AST, K, V> extends IContext {
  IFn findFn(Class cls, String name);

  IFn findFn(Class cls, String name, int args);

  Object eval(AST ast);

  Object eval(AST ast, IEnv env);

  IEnv<K, V> getEnv();

  V getObj(K key);

  V setObj(K key, V value);

  AST readString(String input);

  Class classFor(String name);

  ClassLoader classLoader();

  IColl<Entry<String, Class>> classCache();

  IContext getRoot();

  IColl<URL> pathCache();

  IColl<URL> pathAdd(URL url);

  IColl<URL> pathAdd(String[] paths);

  IColl<URL> pathRemove(String[] paths);

  Class aliasAdd(K key, Class v);

  Class aliasRemove(K key);

  IColl<Entry<String, Class>> aliasCache();
}
