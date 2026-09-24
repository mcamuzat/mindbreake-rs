import { EngineClient, randomSeed } from "../engine/client";
import type { GameAction, GameView } from "../engine/types";
import { Session } from "./session";

const HUMAN = 0;
const AI = 1;

/** Against the Monte Carlo AI, with the engine in a local worker. */
export class LocalAiSession extends Session {
  readonly opponentLabel = "IA";
  readonly canRestart = true;
  /** Simulated games per action: the AI's difficulty. */
  playouts = 150;

  private engine = new EngineClient();
  private aiTimer: ReturnType<typeof setTimeout> | undefined;

  constructor(private sets: string[]) {
    super();
    this.restart();
  }

  restart() {
    clearTimeout(this.aiTimer);
    this.engine.newGame(randomSeed(), HUMAN, this.sets).then((r) => this.show(r.view), (e) => this.fail(e));
  }

  act(action: GameAction) {
    this.engine.act(HUMAN, action).then(
      (r) => this.show(r.view),
      (e) => this.update({ notice: String(e) }),
    );
  }

  private show(view: GameView) {
    const aiToAct = view.toAct === AI;
    this.update({ view, thinking: aiToAct, notice: null });
    if (aiToAct) {
      // A short pause so the human can follow the AI's moves.
      this.aiTimer = setTimeout(() => {
        this.engine.aiStep(AI, this.playouts, HUMAN).then((r) => this.show(r.view), (e) => this.fail(e));
      }, 500);
    }
  }

  dispose() {
    clearTimeout(this.aiTimer);
    this.engine.dispose();
  }
}
