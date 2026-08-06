;;;; Benchmark runner for the lisp-hara comparison suite (SBCL).
;;;; Contract mirrors lib/bench/luajit-hara/lua_runner.lua:
;;;;   sbcl --script sbcl_runner.lisp MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS
;;;; Reads + evals the source on every call (matching hara's eval_native
;;;; per-call semantics; SBCL's default *evaluator-mode* is :compile) and
;;;; prints one JSON line:
;;;;   {"runtime":"sbcl","workload":"ID","first_ns":N,"samples_ns":[...]}

(defparameter *args* (cdr sb-ext:*posix-argv*))

(when (/= (length *args*) 6)
  (format *error-output* "sbcl_runner expects MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS~%")
  (sb-ext:exit :code 2))

(destructuring-bind (mode id source-hex expected windows-s calls-s) *args*
  (let ((windows (parse-integer windows-s :junk-allowed t))
        (calls (parse-integer calls-s :junk-allowed t)))
    (unless (and windows calls)
      (format *error-output* "~a: invalid windows/calls~%" id)
      (sb-ext:exit :code 2))
    (flet ((fail (message)
             (format *error-output* "~a: ~a~%" id message)
             (sb-ext:exit :code 1))
           (hex-decode (s)
             (let* ((n (length s))
                    (out (make-string (floor n 2))))
               (loop for i from 0 below n by 2
                     do (setf (char out (floor i 2))
                              (code-char (parse-integer s :start i :end (+ i 2)
                                                          :radix 16))))
               out))
           (clock-ns ()
             (round (* (/ (get-internal-run-time) internal-time-units-per-second)
                       1d9))))
      (let* ((source (hex-decode source-hex))
             (form (read-from-string source))
             (prepared (when (string= mode "prepared")
                         (compile nil `(lambda () ,form))))
             (eval-once
               (lambda ()
                 (let ((value (if prepared (funcall prepared)
                                  (eval (read-from-string source)))))
                   (unless (string= (princ-to-string value) expected)
                     (fail (format nil "expected ~a, got ~a" expected value))))))
             (started (clock-ns)))
        (funcall eval-once)
        (let ((first-ns (- (clock-ns) started))
              (samples '()))
          (dotimes (w windows)
            (let ((window-started (clock-ns)))
              (dotimes (c calls) (funcall eval-once))
              (push (round (/ (- (clock-ns) window-started) calls)) samples)))
          (format t "{\"runtime\":\"sbcl\",\"workload\":\"~a\",\"first_ns\":~a,\"samples_ns\":[~{~a~^,~}]}~%"
                  id first-ns (nreverse samples)))))))
