#ifndef NH_EVENTS_H
#define NH_EVENTS_H

/*
 * Le bus d'événements : ce qui vient de se passer dans le monde, dit UNE fois, à l'endroit où
 * ça arrive (nh_world_compromise, nh_grant_xp, nh_grant_reputation…), sans que l'émetteur sache
 * qui s'y intéresse. Les quêtes et le déblocage des contacts y sont abonnés. Le tutoriel garde
 * son propre crochet (nh_tutorial_on_command) : il réagit à la commande tapée, pas à ses effets.
 *
 * Deux choix qui comptent :
 *
 *  - DIFFÉRÉ. nh_event() ne fait qu'enregistrer ; c'est nh_events_flush(), appelée par
 *    nh_dispatch() une fois la commande terminée, qui prévient les abonnés. Ainsi
 *    « OBJECTIF ACCOMPLI » s'affiche APRÈS le résultat de la commande et non au milieu de ses
 *    lignes, et un abonné peut lui-même émettre (une quête terminée donne de l'expérience, qui
 *    fait monter de niveau…) sans jamais rappeler un abonné en pleine exécution.
 *
 *  - Les abonnés relisent l'ÉTAT du jeu (niveau, systèmes compromis…) au lieu de compter les
 *    événements. La valeur transportée est une précision utile aux tests et aux futurs abonnés ;
 *    perdre un événement (file pleine) ne fausse donc rien. Ce que les abonnés ne peuvent pas
 *    deviner — une commande vient de s'achever, donc le temps a passé et l'alerte a bougé —
 *    est lui-même un événement (NH_EV_COMMAND).
 *
 * L'état du bus fait partie de GameState mais n'est jamais sauvegardé : il est vide entre deux
 * commandes.
 */

#include <stdbool.h>

struct GameState;

typedef enum
{
    NH_EV_COMMAND,          /* une commande vient de s'achever (value : NhDispatch) */
    NH_EV_NODE_COMPROMISED, /* un système est piraté pour la première fois (value : indice, voir NhNode) */
    NH_EV_FILES_EXTRACTED,  /* des fichiers viennent d'être extraits (value : combien) */
    NH_EV_LEVEL_UP,         /* un niveau de plus (value : le nouveau niveau) */
    NH_EV_REPUTATION,       /* la réputation a changé (value : le nouveau total) */
    NH_EV_ITEM_BOUGHT,      /* achat en boutique (value : ShopItemType) */
    NH_EV_CONTACT_MET,      /* conversation avec un contact (value : ContactType) */
    NH_EV_MILESTONE,        /* récompense unique obtenue (value : NhMilestone) */
    NH_EV_QUEST_COMPLETED,  /* une quête est terminée (value : QuestType) */
    NH_EV_COUNT
} NhEvent;

typedef struct
{
    NhEvent type;
    int value;
} NhEventRecord;

#define NH_EVENT_QUEUE 32

/* Espion appelé pour chaque événement livré, avant les abonnés (tests, diagnostic). */
typedef void (*NhEventObserver)(void *ctx, const NhEventRecord *event);

typedef struct
{
    NhEventRecord queue[NH_EVENT_QUEUE]; /* file circulaire */
    int head;
    int count;
    int dropped;                         /* événements perdus : file pleine ou boucle coupée */
    bool flushing;                       /* nh_events_flush() est en cours */
    unsigned emitted[NH_EV_COUNT];       /* total émis, perdus compris */
    NhEventObserver observer;
    void *observer_ctx;
} EventBus;

/* Enregistre un événement ; ne prévient personne avant nh_events_flush(). Type inconnu : ignoré. */
void nh_event(struct GameState *gs, NhEvent event, int value);

/*
 * Livre les événements en attente, dans l'ordre, à chaque abonné. Les événements émis PENDANT la
 * livraison sont livrés dans la foulée, par la même boucle (pas de récursion). Sans effet si
 * elle est déjà en cours.
 */
void nh_events_flush(struct GameState *gs);

/* Oublie les événements en attente (partie terminée : plus personne à prévenir). */
void nh_events_clear(struct GameState *gs);

/* Nombre d'événements en attente, et lecture du n-ième (0 = le plus ancien) sans le retirer. */
int nh_events_pending(const struct GameState *gs);
bool nh_events_peek(const struct GameState *gs, int index, NhEventRecord *out);

/* Combien de fois `event` a été émis depuis la création de la partie (0 si type inconnu). */
unsigned nh_events_emitted(const struct GameState *gs, NhEvent event);

#endif /* NH_EVENTS_H */
