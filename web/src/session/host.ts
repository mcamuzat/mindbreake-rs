import Peer, { type DataConnection } from "peerjs";
import { EngineClient, randomSeed } from "../engine/client";
import type { GameAction, GameView } from "../engine/types";
import {
  PEER_PREFIX,
  PROTOCOL_VERSION,
  newGameCode,
  parseGuestMessage,
  type HostMessage,
} from "./protocol";
import { Session } from "./session";

const HOST = 0;
const GUEST = 1;

/**
 * Hosts a game: the engine runs here and is the authority. The guest sends
 * its actions, the engine applies them on its behalf (seat 1), and each side
 * receives only its own filtered view.
 */
export class HostSession extends Session {
  readonly opponentLabel = "Adversaire";
  readonly canRestart = true;

  private engine = new EngineClient();
  private peer: Peer | null = null;
  private guest: DataConnection | null = null;
  private started = false;

  constructor(private sets: string[]) {
    super();
    this.update({ status: "Création de la partie…" });
    this.listen(newGameCode());
  }

  private listen(code: string) {
    const peer = new Peer(PEER_PREFIX + code);
    this.peer = peer;
    peer.on("open", () =>
      this.update({ shareCode: code, status: "En attente de ton adversaire : partage-lui le code ou le lien." }),
    );
    peer.on("connection", (conn) => this.accept(conn));
    peer.on("error", (err) => {
      if (err.type === "unavailable-id") {
        // Code already taken: try another one.
        peer.destroy();
        this.listen(newGameCode());
      } else if (err.type !== "peer-unavailable") {
        this.fail(`Connexion : ${err.message}`);
      }
    });
    // Signaling server lost: already established connections keep working.
    peer.on("disconnected", () => {
      if (!peer.destroyed) peer.reconnect();
    });
  }

  private accept(conn: DataConnection) {
    if (this.guest?.open) {
      conn.on("open", () => {
        this.send({ type: "rejected", reason: "La partie est déjà complète." }, conn);
        setTimeout(() => conn.close(), 500);
      });
      return;
    }
    this.guest = conn;
    conn.on("data", (data) => void this.onGuestMessage(conn, data));
    conn.on("close", () => {
      if (this.guest !== conn) return;
      this.guest = null;
      this.update({
        thinking: false,
        status: "Ton adversaire s'est déconnecté. Il peut revenir avec le même lien.",
      });
    });
  }

  private async onGuestMessage(conn: DataConnection, data: unknown) {
    if (conn !== this.guest) return;
    const msg = parseGuestMessage(data);
    if (!msg) {
      this.send({ type: "error", message: "Message invalide." });
      return;
    }
    try {
      switch (msg.type) {
        case "hello": {
          if (msg.version !== PROTOCOL_VERSION) {
            this.send({ type: "rejected", reason: `Versions différentes (hôte ${PROTOCOL_VERSION}, toi ${msg.version}).` });
            setTimeout(() => conn.close(), 500);
            return;
          }
          // First connection: start the game. Reconnection: resume it.
          if (!this.started) {
            await this.engine.newGame(randomSeed(), HOST, this.sets);
            this.started = true;
          }
          const theirs = await this.engine.view(GUEST);
          this.send({ type: "welcome", version: PROTOCOL_VERSION, view: theirs.view });
          this.update({ status: null });
          await this.broadcast();
          break;
        }
        case "action":
          // The engine checks it is the guest's decision and that it is legal.
          await this.engine.act(GUEST, msg.action);
          await this.broadcast();
          break;
      }
    } catch (e) {
      this.send({ type: "error", message: e instanceof Error ? e.message : String(e) });
    }
  }

  act(action: GameAction) {
    this.engine.act(HOST, action).then(
      () => this.broadcast(),
      (e) => this.update({ notice: String(e) }),
    );
  }

  restart() {
    if (!this.guest?.open) return;
    this.engine
      .newGame(randomSeed(), HOST, this.sets)
      .then(() => this.broadcast())
      .catch((e) => this.fail(e));
  }

  /** Sends each player their own view of the current state. */
  private async broadcast() {
    const [mine, theirs] = await Promise.all([this.engine.view(HOST), this.engine.view(GUEST)]);
    this.show(mine.view);
    this.send({ type: "view", view: theirs.view });
  }

  private show(view: GameView) {
    this.update({ view, notice: null, thinking: view.toAct === GUEST });
  }

  private send(message: HostMessage, conn: DataConnection | null = this.guest) {
    if (conn?.open) void conn.send(message);
  }

  dispose() {
    this.peer?.destroy();
    this.engine.dispose();
  }
}
