# Règles implémentées

Sources :
- **[R]** le livret officiel de *First Contact*, via le [résumé RulesPal](https://www.rulespal.com/mindbug-first-contact/rulebook), et le [livret PDF](https://cdn.1j1ju.com/medias/24/8c/40-mindbug-rulebook.pdf) ;
- **[FAQ]** la [FAQ officielle](https://mindbug.me/faq/).

Les points marqués ❓ ne sont tranchés par aucune de ces sources.

## Mise en place [R]
- Chaque joueur commence avec 3 points de vie et 2 Mindbugs.
- Chacun reçoit 10 cartes comme pioche personnelle, puis en pioche 5 pour former sa main de départ.
- Le premier joueur est tiré au hasard.

## Tour
- À son tour, on fait **une seule** action : jouer une carte, **ou** attaquer avec une créature. [R]
- **Dès qu'une carte quitte ta main** (jouée, défaussée ou volée), tu pioches jusqu'à en avoir 5. Si ta pioche est vide, tu ne pioches plus. [R]
- Un joueur qui ne peut ni jouer ni attaquer perd la partie. [FAQ]
- On gagne dès que l'adversaire tombe à 0 point de vie. [R]

## Mindbug [R]
- Quand un joueur joue une carte **de sa main**, son adversaire peut dépenser un Mindbug pour la jouer à sa place.
- La carte entre alors dans la zone de jeu de l'adversaire, et c'est pour lui que l'effet *Play* se déclenche.
- Le joueur qui s'est fait voler sa carte finit son tour, puis joue immédiatement un tour supplémentaire.
- Une carte jouée par un effet (depuis une défausse, par exemple) ne peut pas être Mindbuggée.

## Combat [R]
- Le défenseur peut bloquer avec une de ses créatures, ou ne pas bloquer.
- S'il ne bloque pas, il perd 1 point de vie.
- S'il bloque, la créature la plus faible est vaincue. En cas d'égalité de puissance, les deux le sont.
- Une créature vaincue va dans la défausse de son **contrôleur actuel**, et c'est lui qui profite de son effet *Defeated*. [FAQ]

## Mots-clés
| Mot-clé | Règle |
|---|---|
| Frenzy [R] | Si elle survit à sa première attaque du tour, elle peut attaquer une seconde fois. |
| Hunter [R][FAQ] | L'attaquant peut choisir la créature ennemie qui doit bloquer, **n'importe laquelle** : les restrictions Sneaky et « ne peut pas être bloquée par » ne s'appliquent pas. Il peut aussi renoncer à choisir, et le défenseur bloque alors normalement. |
| Poisonous [R] | En combat, elle vainc toujours la créature ennemie, quelle que soit sa puissance. |
| Sneaky [R] | Elle ne peut être bloquée que par des créatures Sneaky. |
| Tough [R] | Si elle devrait être vaincue et n'est pas épuisée, elle est épuisée à la place. Épuisée, elle peut toujours attaquer, bloquer et utiliser ses capacités. Un Tough épuisé qui affronte une créature Poisonous plus forte est bien vaincu. [FAQ] |

## Choix
- C'est le joueur qui accomplit l'action qui fait les choix. [FAQ] Par exemple, sur « L'adversaire défausse 2 cartes », c'est l'adversaire qui choisit ses cartes.
- ❓ « Défausser » et « vaincre N créatures » : on choisit une carte à la fois. Une capacité « jusqu'à N » peut s'arrêter plus tôt, avec l'action `Done`.

## Interprétations (❓)
- ❓ Si les deux joueurs tombent à 0 point de vie en même temps, le joueur actif gagne (règle maison).
- ❓ Quand plusieurs effets se déclenchent en même temps, ils se résolvent dans l'ordre où ils ont été déclenchés. Selon le livret, c'est le joueur actif qui choisit cet ordre : ce choix n'est pas encore implémenté.
- ❓ « Créatures avec une puissance de N ou moins » dans un **bonus de puissance** se compare à la puissance imprimée, pour éviter une définition circulaire. Pour un octroi de mot-clé (Snail Thrower), c'est la puissance courante qui compte.
- ❓ Sharky Crab-Dog-Mummypus copie les mots-clés **imprimés ou octroyés** de l'ennemi, mais pas ceux que l'ennemi aurait lui-même copiés (sinon, deux Sharky face à face se copieraient l'un l'autre à l'infini).
- ❓ La carte Short-Neck Giraffodile semble remplacer Giraffodile dans les réimpressions : elle n'est pas dans le pool par défaut.

## Extensions (New Servants, Beyond Evolution, promos)
Tous les points de cette section sont ❓ : ni le livret de *First Contact* ni la FAQ ne les couvrent, et les règles ont été déduites du texte des cartes.
- ❓ **Action** : un joueur peut, à la place de jouer une carte ou d'attaquer, activer la capacité *Action* d'une de ses créatures. Ce n'est pas limité à une fois par créature, puisque le tour ne comporte qu'une action.
- ❓ **Evolve** : la carte est remplacée par le stade suivant de sa ligne (elle garde son épuisement et reste en jeu). Seul le premier stade est dans la pioche ; les suivants sont dans le set `Evolutions` (0 exemplaire). Une carte qui quitte le jeu redevient son premier stade.
- ❓ « Vous pouvez… Si vous le faites, … » : la seconde partie ne se joue que si la première a été accomplie ; « pour chaque… » se répète autant de fois qu'elle l'a été.
- ❓ **Restrictions** (« ne peut pas attaquer / bloquer ») : un Hunter ne peut pas choisir comme bloqueur une créature qui ne peut pas bloquer. Les formules « ne peut pas être bloquée par… » (Elephantopus, Mole Machine, Watts Dog) restent contournées par Hunter, comme dans la FAQ.
- ❓ Un joueur qui ne peut ni jouer, ni attaquer, ni activer d'*Action* (mains vide, créatures toutes empêchées d'attaquer) perd la partie.
- ❓ **Captain Hippo** : tant qu'il est en jeu, l'adversaire doit attaquer avec un Hunter s'il en a un, et ce Hunter doit choisir Hippo comme bloqueur.
- ❓ **Effets « jusqu'à la fin du tour »** (Blastfish, The Lurker, Turf The Surfer) : effacés à la fin de chaque tour, y compris un tour supplémentaire.
- ❓ **Fin de tour** : les capacités « à la fin du tour » se déclenchent pour toutes les créatures en jeu, celles du joueur actif d'abord. One-Eye Felix compte les créatures qu'il a vaincues **en combat**.
- ❓ **Pile inutilisée** : les cartes non distribuées (Future Eric). Le dessus de la pile est la dernière carte ; elles sont glissées sous la pioche.
- ❓ **Alien Brain** : n'empêche que les effets qui mettent une carte en main (vol, reprise, échange, don). La pioche automatique jusqu'à 5 cartes n'est pas concernée.
- ❓ **Mindbug Bug** : l'adversaire perd la vie avant d'utiliser le Mindbug ; s'il meurt, le Mindbug n'est pas utilisé.
- ❓ **Alpacoodle** : les créatures écartées reviennent chez leur dernier contrôleur, épuisement compris. Elles ne sont pas visibles dans l'interface.
- ❓ Perdre de la vie inclut « la vie devient N » (Turbo Bug) : Hyenix se déclenche dans ce cas aussi.
- Non implémentés : *Beyond Eternity* (Boost, « en défausse »), *Battlefruit* (Harvest, Octonite, Fast), *Tag Team*, *King of Tokyo*, et les promos 2023 utilisant Boost ou « en défausse ».
