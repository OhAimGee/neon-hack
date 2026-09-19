#ifndef NH_INTRO_H
#define NH_INTRO_H

/*
 * Prologue d'une nouvelle partie : le réveil devant le cyberdeck, l'appel d'ECHO-7, le choix du
 * nom du héros (« Case » par défaut) puis la proposition de suivre le tutoriel ou de se
 * débrouiller seul. Le nom se choisit donc dans la fiction — ECHO-7 le demande, le confirme, le
 * « grave dans le réseau » — et non dans un formulaire.
 *
 * Doit tourner AVANT le démarrage de l'interface fixe (qui repousse le texte déjà affiché dans
 * l'historique du terminal) ; la première consigne du tutoriel est annoncée après, voir
 * nh_tutorial_announce().
 */

#include "game.h"

typedef enum
{
    NH_INTRO_OK,
    NH_INTRO_EOF /* entrée fermée en cours de route : rien n'est fixé, la partie n'a pas commencé */
} NhIntroResult;

/*
 * `gs` doit être un état neuf (init_game). Sur NH_INTRO_OK, le nom est choisi et le tutoriel est
 * démarré (ou passé, au choix du joueur).
 */
NhIntroResult nh_intro_run(GameState *gs);

#endif /* NH_INTRO_H */
