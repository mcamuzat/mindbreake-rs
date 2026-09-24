// Host ↔ guest P2P protocol. The host runs the engine (the authority); the
// guest only sends actions and displays the views it receives.
import type { GameAction, GameView } from "../engine/types";

/** Bumped on any incompatible change: host and guest must match. */
export const PROTOCOL_VERSION = 3;

/** Prefix of PeerJS ids, so a short code doesn't collide with other apps. */
export const PEER_PREFIX = "mindbreake-rs-";

export type GuestMessage =
  | { type: "hello"; version: number }
  | { type: "action"; action: GameAction };

export type HostMessage =
  | { type: "welcome"; version: number; view: GameView }
  | { type: "view"; view: GameView }
  /** Action refused: the game goes on. */
  | { type: "error"; message: string }
  /** Connection refused: the guest must leave. */
  | { type: "rejected"; reason: string };

function isObject(data: unknown): data is Record<string, unknown> {
  return typeof data === "object" && data !== null;
}

/**
 * The host does not trust anything coming from the network: shape check here,
 * and the engine itself validates the action (legality and whose turn it is).
 */
export function parseGuestMessage(data: unknown): GuestMessage | null {
  if (!isObject(data)) return null;
  if (data.type === "hello" && typeof data.version === "number") {
    return { type: "hello", version: data.version };
  }
  if (data.type === "action" && isObject(data.action) && typeof data.action.type === "string") {
    return { type: "action", action: data.action as GameAction };
  }
  return null;
}

export function parseHostMessage(data: unknown): HostMessage | null {
  if (!isObject(data) || typeof data.type !== "string") return null;
  return data as HostMessage;
}

/** Short code to share, without ambiguous characters (0/O, 1/I…). */
export function newGameCode(): string {
  const alphabet = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const bytes = crypto.getRandomValues(new Uint8Array(6));
  return Array.from(bytes, (b) => alphabet[b % alphabet.length]).join("");
}

export function normalizeGameCode(code: string): string {
  return code.trim().toUpperCase();
}
