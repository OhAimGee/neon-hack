#ifndef NH_COMPLETE_H
#define NH_COMPLETE_H

/*
 * Ce que la touche TAB propose à l'invite de commandes (voir ui/lineedit.h pour la mécanique) :
 *
 *   - au premier mot : les commandes que le joueur peut utiliser MAINTENANT (mêmes règles que
 *     `help` et que la barre du bas : niveau, déblocage, pas les commandes cachées). Un alias
 *     n'est proposé que s'aucun nom officiel ne convient (« exi » → exit) ;
 *   - à l'argument, selon NhCommand.arg : les systèmes déjà découverts par `scan`, les contacts
 *     débloqués, les numéros de messages reçus. Rien pour une commande sans argument ni pour un
 *     texte libre (decrypt).
 *
 * Le joueur ne se voit donc jamais proposer ce qu'il ne peut pas utiliser, et ne découvre rien
 * qu'il n'a pas déjà trouvé dans le jeu.
 */

#include "../ui/lineedit.h"

/* À passer à nh_lineedit_read(), avec le `GameState *` comme contexte. */
void nh_complete_line(void *gs, const char *before, NhCompletions *out);

#endif /* NH_COMPLETE_H */
