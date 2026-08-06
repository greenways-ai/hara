(ns benchmark.runtime
  (:require [clojure.string :as string])
  (:import [java.util Base64]))

(defn decode [value]
  (String. (.decode (Base64/getUrlDecoder) value) "UTF-8"))

(defn json-string [value]
  (str "\"" (.replace (str value) "\"" "\\\"") "\""))

(defn main []
  (let [[runtime id encoded-source expected windows calls] *command-line-args*
        source (decode encoded-source)
        windows (Long/parseLong windows)
        calls (Long/parseLong calls)
        evaluate #(load-string source)
        first-start (System/nanoTime)
        first-value (evaluate)
        first-ns (- (System/nanoTime) first-start)]
    (when (not= expected (pr-str first-value))
      (throw (ex-info "benchmark checksum mismatch"
                      {:id id :expected expected :actual (pr-str first-value)})))
    (let [samples
          (loop [window 0 result []]
            (if (= window windows)
              result
              (let [started (System/nanoTime)]
                (dotimes [_ calls]
                  (when (not= expected (pr-str (evaluate)))
                    (throw (ex-info "benchmark checksum changed" {:id id}))))
                (recur (inc window)
                       (conj result (quot (- (System/nanoTime) started) calls))))))]
      (println
       (str "{\"runtime\":" (json-string runtime)
            ",\"workload\":" (json-string id)
            ",\"first_ns\":" first-ns
            ",\"samples_ns\":[" (string/join "," samples) "]}")))))

(main)
