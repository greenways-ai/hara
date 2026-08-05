import { PONG_SOURCE } from "./pong.js";

/**
 * Curated demo snippets for the live card.
 * Each entry: { id, title, kind: "console" | "canvas", source }.
 */

const FIRST_EVAL = `(+ 19 23)`;

const COLLECTIONS = `(def scores [4 8 15 16 23 42])

(map (fn [score] (* score 2)) scores)

(filter (fn [score] (> score 10)) scores)`;

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
              (conj (get board side) pos))]
        (let [line (winning-condition (get new-board side))
              is-full (= 0 (count (get new-board :bg)))]
          (let [is-winner (not (nil? line))]
            {:board new-board
             :turn (if (= side :p1) :p2 :p1)
             :status (if is-winner :done (if is-full :done :active))
             :winner (if is-winner side (if is-full :draw nil))
             :winning-line line}))))))

(next-move (new-game) [:p1 :bb])`;

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
  { id: "canvas-pong", title: "Pong", kind: "canvas", source: PONG_SOURCE }
];

/**
 * @param {string} id
 * @returns {LiveSnippet | null}
 */
export function getLiveSnippet(id) {
  return LIVE_SNIPPETS.find((snippet) => snippet.id === id) ?? null;
}
