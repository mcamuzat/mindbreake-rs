import Peer, { type DataConnection } from "peerjs";
import type { GameAction } from "../engine/types";
import {
  PEER_PREFIX,
  PROTOCOL_VERSION,
  normalizeGameCode,
  parseHostMessage,
  type GuestMessage,
} from "./protocol";
import { Session } from "./session";

/**
 * Joins a game hosted by someone else. No engine here: actions go to the
 * host, which sends back our filtered view (we never see the opponent's hand).
 */
export class GuestSession extends Session {
  readonly opponentLabel = "Hôte";
  readonly canRestart = false;

  private peer = new Peer();
  private conn: DataConnection | null = null;

  constructor(code: string) {
    super();
    const hostId = PEER_PREFIX + normalizeGameCode(code);
    this.update({ status: "Connexion à la partie…" });
    this.peer.on("open", () => this.connect(hostId));
    this.peer.on("error", (err) => {
      this.fail(
        err.type === "peer-unavailable"
          ? "Partie introuvable : vérifie le code, et que l'hôte a toujours la page ouverte."
          : `Connexion : ${err.message}`,
      );
    });
  }

  private connect(hostId: string) {
    const conn = this.peer.connect(hostId, { reliable: true, serialization: "json" });
    this.conn = conn;
    conn.on("open", () => this.send({ type: "hello", version: PROTOCOL_VERSION }));
    conn.on("data", (data) => this.onHostMessage(data));
    conn.on("close", () => {
      if (this.getState().error) return;
      this.update({ thinking: false, status: "Connexion perdue avec l'hôte. Recharge la page pour revenir." });
    });
  }

  private onHostMessage(data: unknown) {
    const msg = parseHostMessage(data);
    if (!msg) return;
    switch (msg.type) {
      case "welcome":
      case "view":
        this.update({
          view: msg.view,
          status: null,
          notice: null,
          thinking: msg.view.winner === null && msg.view.toAct !== msg.view.viewer,
        });
        break;
      case "error":
        this.update({ notice: msg.message, thinking: false });
        break;
      case "rejected":
        this.fail(msg.reason);
        this.conn?.close();
        break;
    }
  }

  act(action: GameAction) {
    this.send({ type: "action", action });
    this.update({ thinking: true });
  }

  private send(message: GuestMessage) {
    if (this.conn?.open) void this.conn.send(message);
  }

  dispose() {
    this.peer.destroy();
  }
}
