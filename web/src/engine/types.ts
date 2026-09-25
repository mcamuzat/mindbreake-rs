// Mirror of the JSON produced by the engine (src/view.rs, src/state.rs).
// Keep in sync with the serde derives on the Rust side.

export type CardId = number;
export type PlayerId = number;

export type Keyword = "Frenzy" | "Hunter" | "Poisonous" | "Sneaky" | "Tough";

export type GameAction =
  | { type: "Play" | "Attack" | "Activate" | "Block" | "Choose"; card: CardId }
  | { type: "UseMindbug" | "PassMindbug" | "NoBlock" | "Done" | "FrenzyAttack" | "EndTurn" | "Accept" | "Decline" };

export type ChoicePurpose =
  | "Defeat"
  | "TakeControl"
  | "Discard"
  | "PlayFromDiscard"
  | "ReturnFromDiscard"
  | "ReturnToHand"
  | "Affect"
  | "MustAttack"
  | "CopyPlayEffect"
  | "GiveCard";

/** "MayDo": accepting resolves an optional effect. "PlayIt": accepting plays the card, declining keeps it in hand. */
export type ConfirmKind = "MayDo" | "PlayIt";

export type WaitingFor =
  | { type: "Action"; player: PlayerId }
  | { type: "Mindbug"; player: PlayerId; creature: CardId }
  | { type: "Block"; chooser: PlayerId; attacker: CardId; candidates: CardId[] }
  | { type: "FrenzyAttack"; player: PlayerId; attacker: CardId }
  | {
      type: "ChooseCards";
      player: PlayerId;
      source: CardId;
      purpose: ChoicePurpose;
      /** Empty when another player is choosing (hidden information). */
      candidates: CardId[];
      remaining: number;
      optional: boolean;
    }
  | { type: "Confirm"; player: PlayerId; source: CardId; kind: ConfirmKind }
  | { type: "GameOver"; winner: PlayerId };

export interface CardView {
  id: CardId;
  name: string;
  power: number;
  basePower: number;
  keywords: Keyword[];
  /** Keywords not printed on the card (granted or copied). */
  grantedKeywords: Keyword[];
  rulesText: string[];
  exhausted: boolean;
}

export interface PlayerView {
  id: PlayerId;
  life: number;
  mindbugs: number;
  /** null for the opponent: hidden information. */
  hand: CardView[] | null;
  handCount: number;
  deckCount: number;
  board: CardView[];
  discard: CardView[];
}

export interface GameView {
  viewer: PlayerId;
  active: PlayerId;
  toAct: PlayerId | null;
  winner: PlayerId | null;
  waiting: WaitingFor;
  legalActions: GameAction[];
  you: PlayerView;
  opponent: PlayerView;
  log: string[];
  keywordHelp: { keyword: Keyword; text: string }[];
}

export interface SetInfo {
  name: string;
  /** Cards it adds to the pool, counting copies. */
  copies: number;
}

export interface AvailableSets {
  /** Fewest cards a pool needs. */
  minCards: number;
  sets: SetInfo[];
}

/** Worker protocol. Game responses carry the view of `seat`. */
export type EngineRequest =
  | { kind: "sets" }
  | { kind: "new"; seed: number; seat: PlayerId; sets: string[] }
  | { kind: "act"; seat: PlayerId; action: GameAction }
  | { kind: "view"; seat: PlayerId }
  | { kind: "ai"; aiSeat: PlayerId; iterations: number; seed: number; seat: PlayerId };

export type EngineResponse =
  | { id: number; ok: true; view: GameView; aiAction: GameAction | null }
  | { id: number; ok: true; available: AvailableSets }
  | { id: number; ok: false; error: string };
