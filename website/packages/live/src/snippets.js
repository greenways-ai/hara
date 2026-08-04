/**
 * Curated demo snippets for the live card.
 * Each entry: { id, title, kind: "console" | "canvas", source }.
 *
 * Console snippets evaluate in the shared kernel session and print a value.
 * Canvas snippets must call (node/start ...) and render frames to the
 * "canvas/background" canvas id — the same contract the docs canvas stages
 * use (see docs/docs/assets/tictactoe/stage-01-canvas.hal).
 */

const FIRST_EVAL = `(+ 19 23)
; => 42`;

const COLLECTIONS = `(def scores [4 8 15 16 23 42])

(map (fn [score] (* score 2)) scores)
; => [8 16 30 32 46 84]

(filter (fn [score] (> score 10)) scores)
; => [15 16 23 42]`;

// Reused from the hara-lang.org landing (website/src/pages/index.astro).
const STATE = `(def game
  (atom {:turn :x
         :moves []}))

(swap! game update :moves conj [1 1])
(deref game)`;

// Rules excerpt from the tictactoe course
// (docs/docs/assets/tictactoe/tictactoe.hal): pure values, no canvas.
const TICTACTOE_MOVE = `(def +winning-conditions+
  [#{:aa :ab :ac} #{:ba :bb :bc} #{:ca :cb :cc}
   #{:aa :ba :ca} #{:ab :bb :cb} #{:ac :bc :cc}
   #{:aa :bb :cc} #{:ac :bb :ca}])

(defn new-game []
  {:board {:bg #{:aa :ab :ac
                 :ba :bb :bc
                 :ca :cb :cc}
           :p1 #{}
           :p2 #{}}
   :turn :p1
   :status :active
   :winner nil
   :winning-line nil})

(defn subset-of? [condition positions]
  (let [cells (vec condition)]
    (loop [index 0]
      (if (< index (count cells))
        (if (contains? positions (nth cells index))
          (recur (inc index))
          false)
        true))))

(defn winning-condition [positions]
  (loop [index 0]
    (if (< index (count +winning-conditions+))
      (let [condition (nth +winning-conditions+ index)]
        (if (subset-of? condition positions)
          condition
          (recur (inc index))))
      nil)))

(defn next-move
  "Transitions from one game state to the next."
  [game move]
  (let [[side pos] move
        board (get game :board)
        turn (get game :turn)
        status (get game :status)]
    (do
      (when (not= status :active)
        (throw (ex-info "Game has finished." {:game game :move move})))
      (when (not= turn side)
        (throw (ex-info (str "Not " side "'s turn.") {:game game :move move})))
      (when (not (contains? (get board :bg) pos))
        (throw (ex-info "Position already taken." {:game game :move move})))
      (let [new-board
            (assoc
              (assoc board :bg (disj (get board :bg) pos))
              side
              (conj (get board side) pos))
            line (winning-condition (get new-board side))
            is-winner (not (nil? line))
            is-full (= 0 (count (get new-board :bg)))]
        {:board new-board
         :turn (if (= side :p1) :p2 :p1)
         :status (if is-winner :done (if is-full :done :active))
         :winner (if is-winner side (if is-full :draw nil))
         :winning-line line}))))

(next-move (new-game) [:p1 :bb])`;

// Minimal adaptation of website/sources/pong.hal: same update rules and
// frame loop, decorative glow/label commands removed.
const CANVAS_PONG = `(ns+
  (:require [studio.draw :as draw]))

;; Minimal autonomous Pong. Both paddles track the ball while it bounces
;; from the viewport edges and the paddle faces.
;; TRY: change paddle-speed, or the initial :vx/:vy, to alter the rally.

(def game-width 1000)
(def game-height 600)
(def ball-radius 12)
(def paddle-width 180)
(def paddle-height 16)
(def paddle-speed 8)
(def top-y 48)
(def bottom-y (- game-height 64))

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
  (let [delta (- target (+ position (/ paddle-width 2)))
        step (clamp delta (- 0 speed) speed)]
    (clamp (+ position step) 0 (- game-width paddle-width))))

(defn update-paddles [state]
  (let [ball-x (get (get state :ball) :x)
        top-x (track-paddle (get state :top-x) ball-x paddle-speed)
        bottom-x (track-paddle (get state :bottom-x) ball-x paddle-speed)]
    (assoc (assoc state :top-x top-x) :bottom-x bottom-x)))

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
  (let [tracked (update-paddles state)
        moved (advance-ball (get tracked :ball))
        side-bounced (bounce-side-walls moved)
        paddle-bounced
        (bounce-paddles side-bounced (get tracked :top-x) (get tracked :bottom-x))]
    (assoc tracked :ball (bounce-screen-walls paddle-bounced))))

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

(defn paddle-commands [view paddle color]
  (let [left (scale-x view (get paddle :x))
        top (scale-y view (get paddle :y))
        width (scale-width view paddle-width)
        height (larger 6 (scale-height view paddle-height))]
    (list [:rect left top width height color 0.96])))

(defn ball-commands [view ball]
  (let [x (scale-x view (get ball :x))
        y (scale-y view (get ball :y))
        radius (larger 6 (scale-width view ball-radius))]
    (list [:circle x y radius "#f5ffff" 1.0])))

(defn append-commands [commands additions]
  (loop [index 0 result commands]
    (if (< index (count additions))
      (recur (inc index) (conj result (nth additions index)))
      result)))

(defn draw-pong-game [state width height]
  (let [view (viewport width height)
        top
        (paddle-commands
          view {:x (get state :top-x) :y top-y} "#ff2e88")
        bottom
        (paddle-commands
          view {:x (get state :bottom-x) :y bottom-y} "#41f5e4")]
    (let [with-top (append-commands (list) top)
          with-bottom (append-commands with-top bottom)
          with-ball
          (append-commands with-bottom (ball-commands view (get state :ball)))]
      with-ball)))

(node/start
  (fn []
    (loop [state (initial-state) tick 0]
      (let [frame (co/await (draw/next-frame "canvas/background"))
            width (get frame "canvas/width")
            height (get frame "canvas/height")
            next-state (if (= (mod tick 2) 0) (update-state state) state)]
        (do
          (co/await
            (draw/render
              "canvas/background"
              {:type :canvas-2d
               :background "#02050b"
               :commands (draw-pong-game next-state width height)}))
          (recur next-state (inc tick)))))))`;

/**
 * @typedef {object} LiveSnippet
 * @property {string} id
 * @property {string} title
 * @property {"console" | "canvas"} kind
 * @property {string} source
 */

/** @type {LiveSnippet[]} */
export const LIVE_SNIPPETS = [
  { id: "first-eval", title: "First eval", kind: "console", source: FIRST_EVAL },
  { id: "collections", title: "Collections", kind: "console", source: COLLECTIONS },
  { id: "state", title: "State", kind: "console", source: STATE },
  { id: "tictactoe-move", title: "Tic-tac-toe", kind: "console", source: TICTACTOE_MOVE },
  { id: "canvas-pong", title: "Pong", kind: "canvas", source: CANVAS_PONG }
];

/**
 * @param {string} id
 * @returns {LiveSnippet | null}
 */
export function getLiveSnippet(id) {
  return LIVE_SNIPPETS.find((snippet) => snippet.id === id) ?? null;
}
