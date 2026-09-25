// The engine runs here, off the UI thread: the AI can search
// without freezing the page. This file only relays calls to WASM.
import init, { aiStep, applyAction, availableSets, newGame, view } from "../wasm/mindbreake.js";
import type { AvailableSets, EngineRequest, EngineResponse, GameAction, GameView } from "./types";

const ready = init();

function handle(
  req: EngineRequest,
): { view: GameView; aiAction: GameAction | null } | { available: AvailableSets } {
  switch (req.kind) {
    case "sets":
      return { available: availableSets() };
    case "new":
      return { view: newGame(req.seed, req.seat, req.sets), aiAction: null };
    case "act":
      return { view: applyAction(req.seat, req.action), aiAction: null };
    case "view":
      return { view: view(req.seat), aiAction: null };
    case "ai": {
      const aiAction: GameAction | null = aiStep(req.aiSeat, req.iterations, req.seed);
      return { view: view(req.seat), aiAction };
    }
  }
}

self.onmessage = async (event: MessageEvent<{ id: number; req: EngineRequest }>) => {
  await ready;
  const { id, req } = event.data;
  let response: EngineResponse;
  try {
    response = { id, ok: true, ...handle(req) };
  } catch (e) {
    response = { id, ok: false, error: e instanceof Error ? e.message : String(e) };
  }
  self.postMessage(response);
};
