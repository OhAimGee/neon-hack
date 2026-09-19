#ifndef NH_TUTORIAL_H
#define NH_TUTORIAL_H

/*
 * Le tutoriel n'est pas un écran à part : c'est la première mission du jeu, « Premiers Pas dans
 * l'Ombre », menée par ECHO-7 dans la vraie boucle de jeu. Le joueur tape de vraies commandes
 * sur de vrais systèmes ; ECHO-7 lui dit quoi faire à chaque étape et constate ce qui a été fait.
 *
 * Fonctionnement : une machine à états très simple (GameState.tutorial.step). nh_dispatch()
 * appelle nh_tutorial_on_command() après CHAQUE commande ; le tutoriel regarde alors si l'étape en
 * cours est accomplie — soit parce que la bonne commande vient d'être lancée (help, scan…), soit
 * parce que l'état du jeu le prouve (niveau 2 atteint, localhost compromis), ce qui rend les
 * étapes robustes à l'ordre dans lequel le joueur explore. Les étapes suivantes déjà satisfaites
 * s'enchaînent d'elles-mêmes. Le joueur reste libre : aucune commande n'est bloquée ; une saisie
 * inconnue, verrouillée ou ratée déclenche simplement un rappel de l'objectif.
 *
 * Le tutoriel n'est PAS actif dans un GameState neuf (init_game) : c'est le prologue (intro.c)
 * qui le démarre ou le passe. Il fait partie de la sauvegarde.
 *
 * Anti-farm : l'état ne peut qu'avancer et la récompense n'est versée qu'à la dernière étape,
 * une fois (`done`). Elle ne donne pas d'expérience : les étapes en rapportent déjà par le jeu.
 */

#include <stdbool.h>

#include "commands.h"
#include "game.h"

typedef enum
{
    NH_TUT_NONE = 0, /* pas de tutoriel en cours */
    NH_TUT_QUESTS,   /* ouvrir le journal de mission */
    NH_TUT_HELP,
    NH_TUT_SCAN,
    NH_TUT_STATUS,
    NH_TUT_LEVELUP,  /* scanner jusqu'au niveau 2 */
    NH_TUT_BRUTEFORCE,
    NH_TUT_LAYLOW,
    NH_TUT_STEP_COUNT
} NhTutStep;

#define NH_TUT_REWARD_CREDITS 100
#define NH_TUT_REWARD_REPUTATION 10

/* Nom d'ECHO-7 tel qu'affiché devant chacune de ses répliques. */
#define NH_ECHO7 "ECHO-7"

bool nh_tutorial_active(const GameState *gs);

/* Lance le tutoriel à sa première étape (sans rien afficher : voir nh_tutorial_announce). */
void nh_tutorial_start(GameState *gs);

/* Le joueur préfère se débrouiller : la mission est close, sans récompense. */
void nh_tutorial_skip(GameState *gs);

/*
 * Annonce l'étape en cours (réplique d'ECHO-7). À appeler au démarrage de la boucle de jeu, une
 * fois l'interface fixe en place (qui repousse tout ce qui a été affiché avant elle). `resumed` :
 * la partie vient d'être rechargée, ECHO-7 salue d'abord le joueur. Sans effet si le tutoriel
 * n'est pas en cours.
 */
void nh_tutorial_announce(const GameState *gs, bool resumed);

/*
 * Crochet de nh_dispatch, appelé après chaque ligne saisie. `command` : nom canonique de la
 * commande exécutée, NULL si elle est inconnue ; `result` : ce que le dispatch a décidé.
 */
void nh_tutorial_on_command(GameState *gs, const char *command, NhDispatch result);

/* Le journal de la mission (commande `quests` pendant le tutoriel). */
void nh_tutorial_print_mission(const GameState *gs);

/*
 * Une réplique d'ECHO-7 : « ECHO-7 » en couleur puis le texte, coupé à la largeur du terminal.
 * `newline` faux : la ligne reste ouverte (une question posée au joueur). `delay_ms` > 0 : effet
 * machine à écrire (sans effet en mode rapide).
 */
void nh_echo_say(const char *text, bool newline, unsigned delay_ms);

#endif /* NH_TUTORIAL_H */
