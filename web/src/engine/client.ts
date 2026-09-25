import type { AvailableSets, EngineRequest, EngineResponse, GameAction, GameView, PlayerId } from "./types";

export interface EngineResult {
  /** The view of the seat named in the request. */
  view: GameView;
  aiAction: GameAction | null;
}

/** Promise-based wrapper around the engine worker. Requests are handled in order. */
export class EngineClient {
  private worker = new Worker(new URL("./engine.worker.ts", import.meta.url), { type: "module" });
  private nextId = 0;
  private pending = new Map<number, { resolve: (r: EngineResponse & { ok: true }) => void; reject: (e: Error) => void }>();

  constructor() {
    this.worker.onmessage = (event: MessageEvent<EngineResponse>) => {
      const res = event.data;
      const waiter = this.pending.get(res.id);
      if (!waiter) return;
      this.pending.delete(res.id);
      if (res.ok) waiter.resolve(res);
      else waiter.reject(new Error(res.error));
    };
  }

  private request(req: EngineRequest): Promise<EngineResponse & { ok: true }> {
    const id = this.nextId++;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker.postMessage({ id, req });
    });
  }

  private async call(req: EngineRequest): Promise<EngineResult> {
    const res = await this.request(req);
    if (!("view" in res)) throw new Error("unexpected engine response");
    return { view: res.view, aiAction: res.aiAction };
  }

  /** The sets a game can be built from. */
  async availableSets(): Promise<AvailableSets> {
    const res = await this.request({ kind: "sets" });
    if (!("available" in res)) throw new Error("unexpected engine response");
    return res.available;
  }

  /** Starts a game from the named sets (the default pool if none) and returns the view of `seat`. */
  newGame(seed: number, seat: PlayerId, sets: string[]) {
    return this.call({ kind: "new", seed, seat, sets });
  }

  /** Plays `action` on behalf of `seat` (the engine checks it is theirs to take). */
  act(seat: PlayerId, action: GameAction) {
    return this.call({ kind: "act", seat, action });
  }

  view(seat: PlayerId) {
    return this.call({ kind: "view", seat });
  }

  /** Lets the AI play for `aiSeat` if the decision is theirs; returns the view of `seat`. */
  aiStep(aiSeat: PlayerId, iterations: number, seat: PlayerId) {
    return this.call({ kind: "ai", aiSeat, iterations, seed: randomSeed(), seat });
  }

  dispose() {
    this.worker.terminate();
  }
}

export function randomSeed(): number {
  return crypto.getRandomValues(new Uint32Array(1))[0];
}
