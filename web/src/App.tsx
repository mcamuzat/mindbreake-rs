// Display only: everything shown comes from the engine's GameView (local or
// sent by the host), and every click sends one of the GameActions the engine
// listed as legal.
import { useEffect, useState, useSyncExternalStore } from "react";
import { cardArt, mindbugArt } from "./art";
import { EngineClient } from "./engine/client";
import type { AvailableSets, CardId, CardView, ChoicePurpose, GameAction, GameView } from "./engine/types";
import { GuestSession } from "./session/guest";
import { HostSession } from "./session/host";
import { LocalAiSession } from "./session/local";
import { normalizeGameCode } from "./session/protocol";
import type { Session } from "./session/session";

const DIFFICULTIES = { Facile: 100, Normal: 1000, Difficile: 4000 } as const;
type Difficulty = keyof typeof DIFFICULTIES;

const CARD_ACTION_LABEL = { Play: "Jouer", Attack: "Attaquer", Activate: "Activer", Block: "Bloquer" } as const;
const CHOICE_LABEL: Record<ChoicePurpose, string> = {
  Defeat: "Vaincre",
  TakeControl: "Prendre",
  Discard: "Défausser",
  PlayFromDiscard: "Jouer",
  ReturnFromDiscard: "Reprendre",
  ReturnToHand: "Renvoyer",
  Affect: "Choisir",
  MustAttack: "Forcer",
  CopyPlayEffect: "Copier",
  GiveCard: "Donner",
};
const CHOICE_PROMPT: Record<ChoicePurpose, string> = {
  Defeat: "choisis une créature à vaincre",
  TakeControl: "choisis une créature à prendre",
  Discard: "choisis une carte à défausser",
  PlayFromDiscard: "choisis une carte de la défausse à jouer",
  ReturnFromDiscard: "choisis une carte de ta défausse à reprendre",
  ReturnToHand: "choisis une créature à renvoyer dans la main",
  Affect: "choisis une créature",
  MustAttack: "choisis la créature qui doit attaquer",
  CopyPlayEffect: "choisis la créature dont copier l'effet de jeu",
  GiveCard: "choisis la carte de ta main à donner",
};
const ACTION_LABEL = {
  UseMindbug: "Utiliser un Mindbug",
  PassMindbug: "Laisser passer",
  NoBlock: "Ne pas bloquer",
  Done: "Terminer",
  FrenzyAttack: "Attaquer à nouveau",
  EndTurn: "Finir le tour",
  Accept: "Oui",
  Decline: "Non",
} as const;

type Mode = { kind: "local"; sets: string[] } | { kind: "host"; sets: string[] } | { kind: "join"; code: string };

function createSession(mode: Mode): Session {
  switch (mode.kind) {
    case "local":
      return new LocalAiSession(mode.sets);
    case "host":
      return new HostSession(mode.sets);
    case "join":
      return new GuestSession(mode.code);
  }
}

/** A `?join=CODE` link joins the game directly. */
function modeFromUrl(): Mode | null {
  const code = new URLSearchParams(window.location.search).get("join");
  return code ? { kind: "join", code: normalizeGameCode(code) } : null;
}

export function App() {
  const [mode, setMode] = useState<Mode | null>(modeFromUrl);

  const quit = () => {
    window.history.replaceState(null, "", window.location.pathname);
    setMode(null);
  };

  return mode ? <GameScreen mode={mode} onQuit={quit} /> : <Menu onStart={setMode} />;
}

const SETS_KEY = "mindbreake.sets";

/** The extensions chosen last time (First Contact, the default pool, at first). */
function savedSets(): string[] {
  try {
    const saved: unknown = JSON.parse(localStorage.getItem(SETS_KEY) ?? "null");
    if (Array.isArray(saved) && saved.every((s) => typeof s === "string")) return saved;
  } catch {
    // Storage unavailable: the default is fine.
  }
  return ["First Contact"];
}

function Menu({ onStart }: { onStart: (mode: Mode) => void }) {
  const [code, setCode] = useState("");
  const [available, setAvailable] = useState<AvailableSets | null>(null);
  const [chosen, setChosen] = useState<string[]>(savedSets);

  // The engine knows which sets exist and how many cards a game needs.
  useEffect(() => {
    const engine = new EngineClient();
    engine.availableSets().then(setAvailable, () => setAvailable(null));
    return () => engine.dispose();
  }, []);

  // A remembered choice may name sets this build does not have: start again from the first one.
  useEffect(() => {
    if (available && !available.sets.some((s) => chosen.includes(s.name))) {
      setChosen(available.sets.slice(0, 1).map((s) => s.name));
    }
  }, [available, chosen]);

  // Only offer what exists (a saved choice may name a set that is gone).
  const sets = available?.sets ?? [];
  const selected = sets.filter((s) => chosen.includes(s.name));
  const cards = selected.reduce((n, s) => n + s.copies, 0);
  const enough = available !== null && cards >= available.minCards;

  const toggle = (name: string) => {
    const next = chosen.includes(name) ? chosen.filter((n) => n !== name) : [...chosen, name];
    setChosen(next);
    try {
      localStorage.setItem(SETS_KEY, JSON.stringify(next));
    } catch {
      // Not remembered: no harm.
    }
  };
  const names = selected.map((s) => s.name);

  return (
    <div className="menu">
      <h1>mindbreake.rs</h1>
      <fieldset className="sets" disabled={available === null}>
        <legend>Extensions</legend>
        {sets.map((s) => (
          <label key={s.name}>
            <input type="checkbox" checked={chosen.includes(s.name)} onChange={() => toggle(s.name)} />
            <span>{s.name}</span>
            <small>{s.copies} cartes</small>
          </label>
        ))}
        {available && (
          <p className={enough ? "count" : "count short"}>
            {enough ? `${cards} cartes en jeu.` : `${cards} cartes : il en faut au moins ${available.minCards}.`}
          </p>
        )}
      </fieldset>
      <button className="primary" disabled={!enough} onClick={() => onStart({ kind: "local", sets: names })}>
        Jouer contre l'IA
      </button>
      <button className="primary" disabled={!enough} onClick={() => onStart({ kind: "host", sets: names })}>
        Héberger une partie
      </button>
      <form
        className="join"
        onSubmit={(e) => {
          e.preventDefault();
          if (code.trim()) onStart({ kind: "join", code: normalizeGameCode(code) });
        }}
      >
        <input value={code} onChange={(e) => setCode(e.target.value)} placeholder="Code de la partie" maxLength={12} />
        <button type="submit" disabled={!code.trim()}>
          Rejoindre
        </button>
      </form>
    </div>
  );
}

function GameScreen({ mode, onQuit }: { mode: Mode; onQuit: () => void }) {
  const [session, setSession] = useState<Session | null>(null);
  const [difficulty, setDifficulty] = useState<Difficulty>("Normal");

  // One session per screen; closed when leaving it (StrictMode-safe).
  useEffect(() => {
    const s = createSession(mode);
    setSession(s);
    return () => s.dispose();
  }, [mode]);

  useEffect(() => {
    if (session instanceof LocalAiSession) session.iterations = DIFFICULTIES[difficulty];
  }, [session, difficulty]);

  if (!session) return null;
  return (
    <Board
      session={session}
      onQuit={onQuit}
      difficulty={session instanceof LocalAiSession ? { value: difficulty, set: setDifficulty } : null}
    />
  );
}

function Board({
  session,
  onQuit,
  difficulty,
}: {
  session: Session;
  onQuit: () => void;
  difficulty: { value: Difficulty; set: (d: Difficulty) => void } | null;
}) {
  const { view, status, notice, error, thinking, shareCode } = useSyncExternalStore(
    session.subscribe,
    session.getState,
  );
  const opponent = session.opponentLabel;
  const [helpOpen, setHelpOpen] = useState(false);

  const header = (
    <header>
      <h1>mindbreake.rs</h1>
      <div className="controls">
        {difficulty && (
          <label>
            IA :{" "}
            <select value={difficulty.value} onChange={(e) => difficulty.set(e.target.value as Difficulty)}>
              {Object.keys(DIFFICULTIES).map((d) => (
                <option key={d}>{d}</option>
              ))}
            </select>
          </label>
        )}
        <button aria-pressed={helpOpen} onClick={() => setHelpOpen(!helpOpen)}>
          Aide
        </button>
        {session.canRestart && view && <button onClick={() => session.restart()}>Nouvelle partie</button>}
        <button onClick={onQuit}>Menu</button>
      </div>
    </header>
  );

  if (error) {
    return (
      <div className="app">
        {header}
        <div className="error">{error}</div>
      </div>
    );
  }

  const shareBanner = shareCode && status && <ShareBanner code={shareCode} status={status} />;

  if (!view) {
    return (
      <div className="app">
        {header}
        <div className="loading">{shareBanner ?? status ?? "Chargement du moteur…"}</div>
      </div>
    );
  }

  const cardActions = (id: CardId) =>
    view.legalActions.filter((a): a is Extract<GameAction, { card: CardId }> => "card" in a && a.card === id);
  const globalActions = view.legalActions.filter(
    (a): a is Exclude<GameAction, { card: CardId }> => !("card" in a),
  );
  const help = Object.fromEntries(view.keywordHelp.map((h) => [h.keyword, h.text]));
  const highlighted = highlightedCard(view);
  const hasActions = (cards: CardView[]) => cards.some((c) => cardActions(c.id).length > 0);
  const act = (a: GameAction) => session.act(a);

  const renderCard = (card: CardView) => (
    <Card
      key={card.id}
      card={card}
      highlighted={card.id === highlighted}
      targetable={cardActions(card.id).some((a) => a.type === "Choose")}
      help={help}
    >
      {cardActions(card.id).map((a) => (
        <button key={a.type} onClick={() => act(a)}>
          {a.type === "Choose" ? choiceLabel(view) : CARD_ACTION_LABEL[a.type]}
        </button>
      ))}
    </Card>
  );

  return (
    <div className="app">
      {header}
      {helpOpen && <KeywordHelpPanel entries={view.keywordHelp} onClose={() => setHelpOpen(false)} />}

      <main>
        {shareBanner || (status && <div className="banner">{status}</div>)}

        <section className="side opponent">
          <PlayerHeader label={opponent} player={view.opponent} active={view.active === view.opponent.id} />
          <div className="row hand-backs">
            {Array.from({ length: view.opponent.handCount }, (_, i) => (
              <div key={i} className="card back" />
            ))}
          </div>
          <div className="row board">{view.opponent.board.map(renderCard)}</div>
          <Discard cards={view.opponent.discard} renderCard={renderCard} open={hasActions(view.opponent.discard)} />
        </section>

        <section className="prompt">
          <div>
            <p>{thinking ? thinkingText(opponent) : prompt(view, opponent)}</p>
            {notice && <p className="notice">{notice}</p>}
          </div>
          <div className="actions">
            {globalActions.map((a) => (
              <button key={a.type} className="primary" onClick={() => act(a)}>
                {ACTION_LABEL[a.type]}
              </button>
            ))}
            {view.winner !== null && session.canRestart && (
              <button className="primary" onClick={() => session.restart()}>
                Rejouer
              </button>
            )}
          </div>
        </section>

        <section className="side you">
          <div className="row board">{view.you.board.map(renderCard)}</div>
          <div className="row hand">{(view.you.hand ?? []).map(renderCard)}</div>
          <Discard cards={view.you.discard} renderCard={renderCard} open={hasActions(view.you.discard)} />
          <PlayerHeader label="Toi" player={view.you} active={view.active === view.you.id} />
        </section>
      </main>

      <aside className="log">
        <h2>Journal</h2>
        <ol reversed>
          {[...view.log].reverse().map((line, i) => (
            <li key={view.log.length - i}>{line}</li>
          ))}
        </ol>
      </aside>
    </div>
  );
}

function KeywordHelpPanel({
  entries,
  onClose,
}: {
  entries: GameView["keywordHelp"];
  onClose: () => void;
}) {
  return (
    <section className="help" aria-label="Aide sur les mots-clés">
      <dl>
        {entries.map((e) => (
          <div key={e.keyword}>
            <dt>{e.keyword}</dt>
            <dd>{e.text}</dd>
          </div>
        ))}
      </dl>
      <button onClick={onClose}>Fermer</button>
    </section>
  );
}

function ShareBanner({ code, status }: { code: string; status: string }) {
  const link = `${window.location.origin}${window.location.pathname}?join=${code}`;
  const [copied, setCopied] = useState(false);
  return (
    <div className="banner share">
      <p>{status}</p>
      <p className="code">{code}</p>
      <div className="share-link">
        <input readOnly value={link} onFocus={(e) => e.target.select()} />
        <button
          onClick={() =>
            navigator.clipboard.writeText(link).then(
              () => setCopied(true),
              () => setCopied(false),
            )
          }
        >
          {copied ? "Copié !" : "Copier le lien"}
        </button>
      </div>
    </div>
  );
}

function Discard({
  cards,
  renderCard,
  open,
}: {
  cards: CardView[];
  renderCard: (c: CardView) => React.ReactNode;
  /** Forced open while one of its cards can be chosen. */
  open: boolean;
}) {
  if (cards.length === 0) return null;
  return (
    <details className="discard" open={open || undefined}>
      <summary>Défausse ({cards.length})</summary>
      <div className="row">{cards.map(renderCard)}</div>
    </details>
  );
}

function PlayerHeader({ label, player, active }: { label: string; player: GameView["you"]; active: boolean }) {
  // Empty pips show the 3 starting life points that are gone.
  const pips = Math.max(3, player.life);
  return (
    <div className={`player-header ${active ? "active" : ""}`}>
      <strong>{label}</strong>
      <span className="pips" title={`${player.life} point(s) de vie`}>
        {Array.from({ length: pips }, (_, i) => (
          <i key={i} className={`pip ${i < player.life ? "" : "off"}`} />
        ))}
      </span>
      <Mindbugs count={player.mindbugs} seat={player.id} />
      <span title="Pioche">Pioche {player.deckCount}</span>
      <span title="Défausse">Défausse {player.discard.length}</span>
    </div>
  );
}

/** The Mindbugs still available, as cards when the images are present. */
function Mindbugs({ count, seat }: { count: number; seat: number }) {
  const art = mindbugArt(seat * 2);
  if (!art) {
    return (
      <span className="bugs" title={`${count} Mindbug(s) restant(s)`}>
        {Array.from({ length: count }, (_, i) => (
          <i key={i} className="bugtok">
            {i + 1}
          </i>
        ))}
        {count === 0 && <span className="none">0 Mindbug</span>}
      </span>
    );
  }
  return (
    <span className="mindbugs" title={`${count} Mindbug(s) restant(s)`}>
      {Array.from({ length: count }, (_, i) => (
        <img key={i} src={mindbugArt(seat * 2 + i) ?? art} alt="Mindbug" />
      ))}
      {count === 0 && <span className="none">0 Mindbug</span>}
    </span>
  );
}

function Card({
  card,
  highlighted,
  targetable,
  help,
  children,
}: {
  card: CardView;
  highlighted: boolean;
  /** One of the cards the engine asks the player to choose. */
  targetable: boolean;
  /** Rules text of each keyword, from the engine. */
  help: Record<string, string>;
  children?: React.ReactNode;
}) {
  const boosted = card.power !== card.basePower;
  const art = cardArt(card.name);
  if (art) {
    // The image already carries the printed text: only what changes in play
    // is overlaid (current power, granted keywords, exhaustion).
    return (
      <div
        className={`card art ${card.exhausted ? "exhausted" : ""} ${highlighted ? "highlighted" : ""} ${targetable ? "targetable" : ""}`}
      >
        <div className="art-frame" title={[card.name, ...card.rulesText].join("\n")}>
          <img src={art} alt={card.name} loading="lazy" />
          {boosted && <span className="power-overlay boosted">{card.power}</span>}
          {card.grantedKeywords.length > 0 && (
            <span className="granted">+ {card.grantedKeywords.join(" · ")}</span>
          )}
        </div>
        <div className="card-actions">{children}</div>
      </div>
    );
  }
  const powerClass = card.power > card.basePower ? "up" : card.power < card.basePower ? "down" : "";
  return (
    <div
      className={`card ${card.exhausted ? "exhausted" : ""} ${highlighted ? "highlighted" : ""} ${targetable ? "targetable" : ""}`}
    >
      <span className={`power ${powerClass}`}>{card.power}</span>
      <span className="name">{card.name}</span>
      {card.keywords.length > 0 && (
        <div className="keywords">
          {card.keywords.map((k) => (
            <span
              key={k}
              className={card.grantedKeywords.includes(k) ? "kw-granted" : undefined}
              title={help[k]}
            >
              {k}
            </span>
          ))}
        </div>
      )}
      <div className="rules">
        {card.rulesText.map((t) => (
          <p key={t}>{t}</p>
        ))}
      </div>
      {card.exhausted && <div className="status">épuisée</div>}
      <div className="card-actions">{children}</div>
    </div>
  );
}

function choiceLabel(view: GameView): string {
  return view.waiting.type === "ChooseCards" ? CHOICE_LABEL[view.waiting.purpose] : "Choisir";
}

function thinkingText(opponent: string): string {
  return opponent === "IA" ? "L'IA réfléchit…" : `${opponent} réfléchit…`;
}

function highlightedCard(view: GameView): CardId | null {
  const w = view.waiting;
  switch (w.type) {
    case "Mindbug":
      return w.creature;
    case "Block":
    case "FrenzyAttack":
      return w.attacker;
    case "ChooseCards":
    case "Confirm":
      return w.source;
    default:
      return null;
  }
}

function prompt(view: GameView, opponent: string): string {
  const w = view.waiting;
  const mine = view.toAct === view.viewer;
  const name = (id: CardId) =>
    [
      ...view.you.board,
      ...view.opponent.board,
      ...(view.you.hand ?? []),
      ...view.you.discard,
      ...view.opponent.discard,
    ].find((c) => c.id === id)?.name ?? "?";
  switch (w.type) {
    case "GameOver":
      return w.winner === view.viewer ? "Victoire !" : `${opponent} gagne.`;
    case "Action":
      return mine ? "À toi : joue une créature ou attaque." : `Tour de ${opponent}.`;
    case "Mindbug":
      return mine
        ? `${opponent} joue ${name(w.creature)}. Utiliser un Mindbug ?`
        : `${opponent} hésite à voler ${name(w.creature)}…`;
    case "Block":
      return mine ? `${name(w.attacker)} attaque : choisis le bloqueur.` : `${name(w.attacker)} attaque.`;
    case "FrenzyAttack":
      return mine
        ? `${name(w.attacker)} peut attaquer une seconde fois (Frenzy).`
        : `${opponent} décide s'il attaque à nouveau.`;
    case "Confirm":
      return mine
        ? w.kind === "PlayIt"
          ? `${name(w.source)} : la jouer (sinon, elle va dans ta main) ?`
          : `${name(w.source)} : utiliser cet effet ?`
        : `${name(w.source)} : ${opponent} décide…`;
    case "ChooseCards":
      return mine
        ? `${name(w.source)} : ${CHOICE_PROMPT[w.purpose]}${w.remaining > 1 ? ` (encore ${w.remaining})` : ""}.`
        : `${name(w.source)} : ${opponent} choisit…`;
  }
}
