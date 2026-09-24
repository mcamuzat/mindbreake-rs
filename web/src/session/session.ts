import type { GameAction, GameView } from "../engine/types";

export interface SessionState {
  /** The local player's view, or null before the game starts. */
  view: GameView | null;
  /** Connection state to show ("waiting for your opponent…"). */
  status: string | null;
  /** Non-fatal message (action refused, etc.). */
  notice: string | null;
  /** Fatal error: the session can no longer continue. */
  error: string | null;
  /** The opponent (AI or remote) is deciding. */
  thinking: boolean;
  /** Code to share to join the game (host only). */
  shareCode: string | null;
}

const INITIAL: SessionState = {
  view: null,
  status: null,
  notice: null,
  error: null,
  thinking: false,
  shareCode: null,
};

/**
 * A game as seen by the page: a state to display, and actions to send.
 * Where the engine runs (here, on the host, …) is the implementation's
 * concern; the UI is the same for all of them.
 */
export abstract class Session {
  abstract readonly opponentLabel: string;
  abstract readonly canRestart: boolean;

  private state: SessionState = INITIAL;
  private listeners = new Set<() => void>();

  abstract act(action: GameAction): void;
  restart(): void {}
  dispose(): void {}

  getState = (): SessionState => this.state;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  };

  protected update(patch: Partial<SessionState>) {
    this.state = { ...this.state, ...patch };
    this.listeners.forEach((l) => l());
  }

  protected fail(error: unknown) {
    this.update({ error: error instanceof Error ? error.message : String(error), thinking: false });
  }
}
