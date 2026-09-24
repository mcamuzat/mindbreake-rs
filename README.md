# mindbreake.rs

Un moteur de règles façon **Mindbug** en Rust, avec la même architecture que phase.rs, en miniature :

- **Cartes typées** (`src/types.rs`, `src/cards.rs`) : mots-clés, déclencheurs (*Play*, *Attack*, *Defeated*), effets composables et bonus statiques. Pas de parser : avec quelques centaines de cartes, on les saisit comme données.
- **Réducteur** (`src/engine.rs`) : `apply(&mut state, action)` vérifie que l'action est légale, puis fait avancer la partie jusqu'à la prochaine décision.
- **État** (`src/state.rs`) : `GameState`, `GameAction`, et `WaitingFor`, qui dit qui doit décider quoi.
- **IA** (`src/ai.rs`) : un agent aléatoire, et un agent Monte Carlo à plat. Pour chaque action possible, il joue N parties au hasard jusqu'au bout. Avant chaque partie simulée, il redistribue les cartes qu'il ne voit pas (sa pioche, la main et la pioche adverses) : c'est la déterminisation.

## Crédits et propriété intellectuelle
mindbreake.rs est un projet non officiel, sans lien avec Nerdlab Games ni avec Richard Garfield.
- **Mindbug** est un jeu de cartes de Richard Garfield, édité par **Nerdlab Games**. Le nom, les règles d'origine, les cartes officielles (*First Contact*, *New Servants*, *Beyond Evolution*, *Battlefruit*, promos…) et leurs illustrations appartiennent à Nerdlab Games et à leurs auteurs.
- Ce dépôt **ne contient ni les cartes officielles, ni leurs textes, ni leurs images** : `src/cards_official.rs`, `data/official-cards.json` et `web/public/official/` sont dans le `.gitignore`, pour un usage personnel uniquement. Les cartes fournies (`src/cards.rs`) sont des créatures originales inventées pour tester le moteur.
- Les textes des cartes et les images utilisés en local viennent de la base communautaire [ryanascherr.github.io/mindbug](https://ryanascherr.github.io/mindbug/) (dépôt [ryanascherr/mindbug](https://github.com/ryanascherr/mindbug)). **Ces images sont les scans officiels de Nerdlab Games** : ce projet ne les redistribue pas, `scripts/fetch-official-art.sh` les télécharge sur la machine de chacun, pour un usage personnel. Les règles suivent le livret officiel et la [FAQ](https://mindbug.me/faq/) de Nerdlab.
- Si vous êtes ayant droit et souhaitez une modification, ouvrez une issue.

## Cartes

- **`src/cards.rs` → `EXAMPLES`** : des créatures originales inventées pour tester le moteur. Elles peuvent être publiées.
- **`src/cards_official.rs`** : les sets officiels *First Contact* (48 cartes, pool par défaut), *New Servants*, *Beyond Evolution* et les promos 2022 à 2024 (`--sets "First Contact,New Servants"` en ligne de commande, `--sets list` pour la liste). Il est **dans le `.gitignore`**, car les cartes appartiennent à Nerdlab Games (usage personnel uniquement). Quand ce fichier existe, `build.rs` active `cfg(official_cards)`, et le jeu l'utilise comme pool par défaut. Sans lui, tout compile et fonctionne avec les cartes d'exemple.
- `data/official-cards.json` (lui aussi dans le `.gitignore`) contient toutes les cartes des 12 sets, extraites de la base de données communautaire [ryanascherr.github.io/mindbug](https://ryanascherr.github.io/mindbug/). Il sert de source pour transcrire les autres sets.
- **Illustrations** : `./scripts/fetch-official-art.sh` télécharge les 269 scans de cartes et les cartes Mindbug depuis le dépôt de la base de données communautaire, dans `web/public/official/` (dans le `.gitignore`, environ 68 Mo), avec un `manifest.json` qui associe chaque nom de carte à son image. La page les utilise si elles sont présentes, et revient aux cartes en texte sinon. Sur une carte illustrée, seul ce qui change en cours de partie est superposé à l'image (puissance modifiée, mots-clés gagnés, épuisement), et la carte s'agrandit au survol.
- `cargo run -- cards` affiche le pool avec le texte de règles **généré à partir des définitions typées**, pour le comparer au texte officiel.

> ⚠️ Avec les cartes officielles, le binaire WASM et `web/dist/` les contiennent : **ne publie pas** ces fichiers.

## Utilisation

```bash
cargo test                          # tests des règles
cargo run --release -- sim 200 30   # Monte Carlo (30 parties simulées/action) contre l'aléatoire, 200 parties
cargo run --release -- play         # toi (joueur 0) contre l'IA Monte Carlo
cargo run --release -- play 42      # même chose, avec une graine fixe
```

## Version web (WASM)

```bash
./scripts/build-wasm.sh    # compile mindbreake-wasm → web/src/wasm/ (≈105 Ko après wasm-opt)
cd web && pnpm install && pnpm dev   # http://localhost:5173
```

Il faut la cible `wasm32-unknown-unknown` (déclarée dans `rust-toolchain.toml`) et `wasm-bindgen-cli`, **exactement dans la même version** que la dépendance `wasm-bindgen` de `mindbreake-wasm/Cargo.toml` (actuellement 0.2.121). `wasm-opt` (binaryen) est optionnel.

```
React (App.tsx) ──postMessage──► Web Worker (engine.worker.ts) ──► WASM (mindbreake-wasm)
      ▲                                                               │ apply_as / view_for
      └──────────────────── GameView (JSON) ◄─────────────────────────┘
```

- **`mindbreake-wasm`** n'est qu'une couche de sérialisation : `newGame`, `view`, `applyAction`, `aiStep`. La partie reste en mémoire WASM (`thread_local!`), et JS ne reçoit que la **vue d'un joueur**.
- **`src/view.rs`** : c'est le moteur qui masque l'information cachée (la main adverse vaut `null`). C'est aussi lui qui calcule la puissance courante, génère le texte de règles et liste les actions légales. Le front ne calcule rien.
- **`apply_as(state, player, action)`** vérifie que c'est bien à ce joueur de décider, en plus de la légalité de l'action.
- Le worker garde la page fluide pendant que l'IA Monte Carlo simule ses parties.
- Le typage TypeScript est écrit à la main dans `web/src/engine/types.ts`, en miroir des `#[serde(...)]` Rust.

## Multijoueur (pair-à-pair)

Menu → **Héberger une partie** : la page affiche un code (et un lien `?join=CODE`) à envoyer à l'adversaire, qui choisit **Rejoindre**.

```
navigateur de l'hôte                          navigateur de l'invité
┌──────────────────────────┐    WebRTC     ┌───────────────────────┐
│ HostSession              │◄── action ────│ GuestSession          │
│  moteur WASM (autorité)  │               │  aucun moteur :       │
│  apply_as(1, action)     │── view(1) ───►│  affiche la vue reçue │
│  view_for(0) → UI hôte   │               │                       │
└──────────────────────────┘               └───────────────────────┘
          signalisation : serveur public PeerJS (0.peerjs.com)
```

C'est le modèle de phase.rs (`P2PHostAdapter`), en miniature :
- **Une seule autorité.** Seul l'hôte fait tourner le moteur. L'invité envoie des `GameAction` et reçoit des `GameView`.
- **Filtrage par le moteur.** L'invité reçoit `view_for(state, 1)` : la main de l'hôte ne quitte jamais son navigateur.
- **Actions vérifiées.** Le protocole vérifie la forme des messages (`web/src/session/protocol.ts`). Le moteur vérifie ensuite, avec `apply_as`, que c'est bien à l'invité de décider et que son action est légale.
- **Versions vérifiées.** `hello` porte `PROTOCOL_VERSION`, et l'hôte refuse une version différente.
- **Reconnexion.** Si l'invité recharge la page (même lien), l'hôte lui renvoie la partie en cours.
- **La même interface pour tous les modes.** `Session` (`web/src/session/`) a trois implémentations, `LocalAiSession`, `HostSession` et `GuestSession`, et l'UI ne sait pas laquelle elle affiche.

Limites :
- L'hôte pourrait tricher en lisant la mémoire de son navigateur, ce qui convient entre amis.
- Si l'hôte ferme son onglet, la partie s'arrête.
- N'importe qui connaissant le code peut prendre la place de l'invité si celui-ci s'est déconnecté.
- Pour jouer à distance, l'adversaire doit pouvoir ouvrir la page. En réseau local, lance `pnpm dev --host`. Sur Internet, il faut héberger `web/dist/`, et avec les cartes officielles, **seulement en privé** (voir plus haut).

## Architecture

```
GameAction ──► apply() ──► legal_actions() contient l'action ?
                             │ oui
                             ▼
                handler (jouer / attaquer / bloquer / Mindbug…)
                             │  met en file les déclenchements (pending)
                             │  et programme l'étape suivante (next_step)
                             ▼
                advance() : effets en attente → étape suivante
                             │  s'arrête dès qu'une décision est nécessaire
                             ▼
                state.waiting = WaitingFor::…
```

- Un effet ciblé (`DefeatChosen`) met la résolution en pause avec `WaitingFor::ChooseTarget`. L'action `Target(id)` la relance là où elle s'était arrêtée.
- **Propriétaire ≠ contrôleur** : une créature volée par un Mindbug change de plateau. Une fois vaincue, elle va dans la défausse de son propriétaire.
- La puissance se calcule toujours avec `GameState::power`, qui applique les bonus statiques. Ne lis jamais `def.power` directement.

## Ajouter une carte

1. Si son effet existe déjà dans `Effect` ou `StaticAbility`, ajoute simplement une entrée à `CATALOG`.
2. Sinon, paramètre un variant existant plutôt que d'en créer un nouveau. Par exemple, élargis `CreatureFilter` au lieu d'ajouter `DefeatEnemyWithPowerAtMost3`.
3. Ajoute un test de règle dans `tests/rules.rs`, construit avec `GameState::empty` et `add_card`.

## Pistes

- ISMCTS (Information Set MCTS) à la place du Monte Carlo à plat : l'IA planifierait plusieurs coups à l'avance au lieu d'un seul.
- Mémoriser les cartes déjà vues (révélées, jouées) pour mieux redistribuer celles qui restent cachées.
- `serde` et une cible WASM pour un front web, comme phase.rs.
- Décider des modes de jeu : draft, extensions, plus de 2 joueurs.

## Publier la page (GitHub Pages, iframe)
`web/dist/` est un site statique (chemins relatifs : il marche dans un sous-dossier). Pour une version **publique**, elle ne doit contenir aucune carte ni image officielle :
- construire depuis un dépôt propre (ou sans `src/cards_official.rs`), donc sans `cfg(official_cards)` : le WASM ne contient alors que les cartes d'exemple ;
- ne pas avoir de `web/public/official/` (`pnpm build` le copie dans `dist/official/` s'il existe) ;
- `./scripts/build-wasm.sh`, puis `cd web && pnpm build`, et publier `web/dist/`.
Les parties à deux passent par le serveur public de PeerJS : il n'y a pas de serveur à héberger.
