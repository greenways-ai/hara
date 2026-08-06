/**
 * Browser-module copy of website/sources/pong.hal.
 * Kept byte-for-byte in sync by website/packages/live/test/snippets.test.mjs.
 */
export const PONG_SOURCE = `(ns+)

(require [studio.draw :as draw])

;; Pong
;; Full-screen autonomous Pong. Both horizontal paddles track the ball while
;; the ball bounces from the viewport edges and the paddle faces.
;; TRY: change paddle-speed to make the rally more or less forgiving.
;; TRY: change the initial vx/vy values to alter the rhythm.

(def game-width 1000)
(def game-height 600)
(def ball-radius 12)
(def paddle-width 180)
(def paddle-height 16)
(def paddle-speed 8)
(def top-y 48)
(def bottom-y (- game-height 64))

(defn smaller [a b] (if (< a b) a b))
(defn larger [a b] (if (> a b) a b))
(defn absolute [value] (if (neg? value) (- 0 value) value))
(defn clamp [value low high]
  (if (< value low) low (if (> value high) high value)))

(defn initial-state []
  {:top-x (/ (- game-width paddle-width) 2)
   :bottom-x (/ (- game-width paddle-width) 2)
   :ball {:x (/ game-width 2)
          :y (/ game-height 2)
          :vx 6
          :vy 5}})

(defn track-paddle [position target speed]
  (let [delta (- target (+ position (/ paddle-width 2)))]
    (let [step (clamp delta (- 0 speed) speed)]
      (clamp (+ position step) 0 (- game-width paddle-width)))))

(defn update-paddles [state]
  (let [ball-x (get (get state :ball) :x)]
    (let [top-x (track-paddle (get state :top-x) ball-x paddle-speed)
          bottom-x (track-paddle (get state :bottom-x) ball-x paddle-speed)]
      (assoc (assoc state :top-x top-x) :bottom-x bottom-x))))

(defn advance-ball [ball]
  (assoc
    (assoc ball :x (+ (get ball :x) (get ball :vx)))
    :y
    (+ (get ball :y) (get ball :vy))))

(defn bounce-side-walls [ball]
  (let [x (get ball :x)
        vx (get ball :vx)]
    (if (<= x ball-radius)
      (assoc (assoc ball :x ball-radius) :vx (absolute vx))
      (if (>= x (- game-width ball-radius))
        (assoc
          (assoc ball :x (- game-width ball-radius))
          :vx
          (- 0 (absolute vx)))
        ball))))

(defn over-paddle [ball paddle-x]
  (let [x (get ball :x)]
    (if (>= x (- paddle-x ball-radius))
      (<= x (+ paddle-x paddle-width ball-radius))
      false)))

(defn bounce-paddles [ball top-x bottom-x]
  (let [y (get ball :y)
        vy (get ball :vy)]
    (if (if (< vy 0)
          (if (<= y (+ top-y paddle-height ball-radius))
            (over-paddle ball top-x)
            false)
          false)
      (assoc
        (assoc ball :y (+ top-y paddle-height ball-radius))
        :vy
        (absolute vy))
      (if (if (> vy 0)
            (if (>= y (- bottom-y ball-radius))
              (over-paddle ball bottom-x)
              false)
            false)
        (assoc
          (assoc ball :y (- bottom-y ball-radius))
          :vy
          (- 0 (absolute vy)))
        ball))))

(defn bounce-screen-walls [ball]
  (let [y (get ball :y)
        vy (get ball :vy)]
    (if (<= y ball-radius)
      (assoc (assoc ball :y ball-radius) :vy (absolute vy))
      (if (>= y (- game-height ball-radius))
        (assoc
          (assoc ball :y (- game-height ball-radius))
          :vy
          (- 0 (absolute vy)))
        ball))))

(defn update-state [state]
  (let [tracked (update-paddles state)]
    (let [moved (advance-ball (get tracked :ball))]
      (let [side-bounced (bounce-side-walls moved)]
        (let [paddle-bounced
              (bounce-paddles side-bounced (get tracked :top-x) (get tracked :bottom-x))]
          (assoc tracked :ball (bounce-screen-walls paddle-bounced)))))))

(defn viewport [width height]
  {:width width :height height :left 0 :top 0})

(defn scale-x [view x]
  (/ (* x (get view :width)) game-width))

(defn scale-y [view y]
  (/ (* y (get view :height)) game-height))

(defn scale-width [view width]
  (/ (* width (get view :width)) game-width))

(defn scale-height [view height]
  (/ (* height (get view :height)) game-height))

(defn field-commands [view]
  ;; The whole browser canvas is the playfield. Keeping this command list
  ;; empty avoids drawing a separate framed or tinted "screen" over the hero.
  (list))

(defn paddle-commands [view paddle color]
  (let [left (scale-x view (get paddle :x))
        top (scale-y view (get paddle :y))
        width (scale-width view paddle-width)
        height (larger 6 (scale-height view paddle-height))]
    (list
      [:rect left (- top 4) width (+ height 8) color 0.12]
      [:rect left top width height color 0.96])))

(defn ball-commands [view ball]
  (let [x (scale-x view (get ball :x))
        y (scale-y view (get ball :y))
        radius (larger 6 (scale-width view ball-radius))]
    (list
      [:circle x y (+ radius 8) "#41f5e4" 0.12]
      [:circle x y radius "#f5ffff" 1.0])))

(defn append-commands [commands additions]
  (loop [index 0 result commands]
    (if (< index (count additions))
      (recur (inc index) (conj result (nth additions index)))
      result)))

(defn draw-pong-game [state width height]
  (let [view (viewport width height)]
    (let [field (field-commands view)
          top
          (paddle-commands
            view {:x (get state :top-x) :y top-y} "#ff2e88")
          bottom
          (paddle-commands
            view {:x (get state :bottom-x) :y bottom-y} "#41f5e4")]
      (let [with-top (append-commands field top)]
        (let [with-bottom (append-commands with-top bottom)]
          (let [with-ball
                (append-commands with-bottom (ball-commands view (get state :ball)))]
            (conj
              with-ball
              [:text "PONG.HAL // AUTOPLAY" 18 (- height 18) "#9ab3c7" 12 0.84])))))))

(node/start
  (fn []
    (loop [state (initial-state)]
      (let [frame (co/await (draw/next-frame "canvas/background"))]
        (let [width (get frame "canvas/width")
              height (get frame "canvas/height")
              next-state (update-state state)]
          (do
            (co/await
              (draw/render
                "canvas/background"
                {:type :canvas-2d
                 :background "#02050b"
                 :commands (draw-pong-game next-state width height)}))
            (recur next-state)))))))
`;
