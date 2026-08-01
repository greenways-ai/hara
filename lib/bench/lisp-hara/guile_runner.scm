;; Benchmark runner for the lisp-hara comparison suite (GNU Guile).
;; Contract mirrors lib/bench/luajit-hara/lua_runner.lua:
;;   guile -s guile_runner.scm ID SOURCE_HEX EXPECTED WINDOWS CALLS
;; Reads + evals the source on every call (matching hara's eval_native
;; per-call semantics) and prints one JSON line:
;;   {"runtime":"guile","workload":"ID","first_ns":N,"samples_ns":[...]}
;;
;; The rnrs import makes make-eqv-hashtable & co. visible to eval'd
;; workload sources in the interaction environment.

(import (rnrs base)
        (rnrs hashtables)
        (rnrs io ports)
        (only (guile) internal-time-units-per-second))

(define args (cdr (command-line)))

(when (not (= (length args) 5))
  (display "guile_runner expects ID SOURCE_HEX EXPECTED WINDOWS CALLS\n" (current-error-port))
  (exit 2))

(define id (list-ref args 0))
(define source-hex (list-ref args 1))
(define expected (list-ref args 2))
(define windows (string->number (list-ref args 3)))
(define calls (string->number (list-ref args 4)))

(unless (and windows calls)
  (display (string-append id ": invalid windows/calls\n") (current-error-port))
  (exit 2))

(define (fail message)
  (display (string-append id ": " message "\n") (current-error-port))
  (exit 1))

(define (hex-decode s)
  (let* ((n (string-length s))
         (out (make-string (quotient n 2))))
    (do ((i 0 (+ i 2)))
        ((>= i n) out)
      (string-set! out (quotient i 2)
                   (integer->char (string->number (substring s i (+ i 2)) 16))))))

(define source (hex-decode source-hex))

(define (clock-ns)
  (round (* 1000000000 (/ (get-internal-run-time) internal-time-units-per-second))))

(define (->string v)
  (call-with-string-output-port (lambda (p) (display v p))))

(define (eval-once)
  (let ((value (eval (read (open-input-string source)) (interaction-environment))))
    (unless (string=? (->string value) expected)
      (fail (string-append "expected " expected ", got " (->string value))))))

(define started (clock-ns))
(eval-once)
(define first-ns (- (clock-ns) started))

(define samples
  (let loop ((w 0) (acc '()))
    (if (>= w windows)
        (reverse acc)
        (let ((window-started (clock-ns)))
          (do ((c 0 (+ c 1)))
              ((>= c calls))
            (eval-once))
          (loop (+ w 1)
                (cons (round (/ (- (clock-ns) window-started) calls)) acc))))))

(display (string-append
          "{\"runtime\":\"guile\",\"workload\":\"" id "\",\"first_ns\":"
          (number->string (round first-ns)) ",\"samples_ns\":["
          (let join ((rest samples))
            (if (null? rest)
                ""
                (string-append (number->string (car rest))
                               (if (null? (cdr rest)) "" (string-append "," (join (cdr rest)))))))
          "]}\n"))
