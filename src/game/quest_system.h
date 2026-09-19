#ifndef NH_QUEST_SYSTEM_H
#define NH_QUEST_SYSTEM_H

/*
 * Les quêtes : des tables `static const` (quest_system.c) qui décrivent chaque quête, et un état
 * minuscule par quête (statut + avancement de chaque objectif). Les textes sont des clés de
 * strings.def : ils suivent la langue et ne sont jamais sauvegardés.
 *
 * Le moteur est « à niveau » : il ne compte pas les événements, il relit l'état du jeu (niveau,
 * systèmes compromis, fichiers extraits, achats…) et en déduit l'avancement. Quand quelque chose
 * change, le bus d'événements (events.h) lui demande simplement de relire (nh_quests_on_event).
 * Conséquences : rien à instrumenter par objectif, une sauvegarde d'avant la quête compte déjà
 * ce qui a été fait, et un événement manqué ne fausse rien. L'avancement ne recule jamais, sauf
 * pour une CONDITION (NH_OBJ_KEEP_ALERT_BELOW), vraie ou fausse à l'instant.
 *
 * Cycle d'une quête : VERROUILLÉE → ACTIVE dès que le niveau et les quêtes prérequises le
 * permettent (annonce à l'écran) → TERMINÉE quand tous ses objectifs sont accomplis
 * simultanément ; les récompenses sont alors versées, une seule fois. Les quêtes principales
 * démarrent d'elles-mêmes ; DISPONIBLE (quête annexe proposée par un contact) et ÉCHOUÉE sont
 * réservées à la suite et ne sont pas encore produites.
 *
 * Le tutoriel est la première quête (QUEST_INTRO_TUTORIAL) : son objectif est « manuel », terminé
 * par tutorial.c avec nh_quest_complete().
 */

#include <stdbool.h>

#include "../i18n/i18n.h"
#include "events.h"

struct GameState;

typedef enum
{
    QUEST_INTRO_TUTORIAL,
    QUEST_FIRST_INFILTRATION,
    QUEST_GATHER_INTEL,
    QUEST_NEXUS_DATA_BREACH,
    QUEST_UNDERGROUND_CONTACT, /* pas encore écrite : phase 4 */
    QUEST_CORPORATE_SABOTAGE,
    QUEST_AI_LIBERATION,
    QUEST_SHADOW_BROKER,
    QUEST_FINAL_SHOWDOWN,
    QUEST_EPILOGUE,
    QUEST_COUNT
} QuestType;

/* Les valeurs sont écrites dans les sauvegardes : ne pas les réordonner. */
typedef enum
{
    QUEST_STATUS_LOCKED,
    QUEST_STATUS_AVAILABLE,
    QUEST_STATUS_ACTIVE,
    QUEST_STATUS_COMPLETED,
    QUEST_STATUS_FAILED
} QuestStatus;

/* Ce que mesure un objectif ; `arg` et `target` de NhObjectiveDef en précisent le sens. */
typedef enum
{
    NH_OBJ_MANUAL,           /* accompli par le code (tutoriel) : ne se mesure pas */
    NH_OBJ_REACH_LEVEL,      /* niveau du joueur >= target */
    NH_OBJ_BUILD_REPUTATION, /* réputation >= target (acquise une fois pour toutes) */
    NH_OBJ_HACK_TARGET,      /* arg = système (NhNode) : compromis ; arg = -1 : nombre de systèmes compromis >= target */
    NH_OBJ_GATHER_DATA,      /* fichiers extraits >= target ; arg = système (NhNode), -1 : tous les systèmes */
    NH_OBJ_MEET_CONTACT,     /* arg = ContactType : au moins une conversation */
    NH_OBJ_PURCHASE_ITEM,    /* arg = masque de ShopItemType (1 << type) : l'un d'eux a été acheté */
    NH_OBJ_DECRYPT_MESSAGE,  /* arg = NhMilestone : jalon obtenu */
    NH_OBJ_KEEP_ALERT_BELOW  /* CONDITION : alerte < arg à l'instant où l'on conclut */
} NhObjective;

#define NH_QUEST_MAX_OBJECTIVES 5
#define NH_QUEST_MAX_PREREQ 2
#define NH_CHAPTER_COUNT 4

typedef struct
{
    NhObjective kind;
    int target; /* valeur à atteindre (1 : simple « fait / pas fait ») */
    int arg;
    NhStr text; /* le seul texte à formater : KEEP_ALERT_BELOW y met `arg` (« sous %d ») */
    bool hidden; /* secret tant qu'il n'est pas accompli : le journal l'annonce sans le dire */
} NhObjectiveDef;

typedef struct
{
    NhStr title;
    NhStr description;
    NhStr lore;
    NhStr location;
    const char *contact; /* nom propre (ECHO-7, R4Z0R…) : jamais traduit */
    int level_required;
    int prerequisites[NH_QUEST_MAX_PREREQ]; /* quêtes à terminer d'abord ; -1 : aucune */
    int chapter;                            /* 1..NH_CHAPTER_COUNT */
    int objective_count;                    /* 0 : quête pas encore écrite, elle reste verrouillée */
    NhObjectiveDef objectives[NH_QUEST_MAX_OBJECTIVES];
    int xp;
    int credits;
    int reputation;
} NhQuestDef;

/* Ce qui change en cours de partie, et donc ce que la sauvegarde retient. */
typedef struct
{
    QuestStatus status;
    int progress[NH_QUEST_MAX_OBJECTIVES]; /* 0..target de chaque objectif */
} QuestState;

typedef struct
{
    QuestState quests[QUEST_COUNT];
} QuestSystem;

/* La définition d'une quête (NULL hors bornes). Une quête pas encore écrite a objective_count == 0. */
const NhQuestDef *nh_quest_def(QuestType quest);

/* État initial : tout verrouillé, sauf le tutoriel qui est actif. */
void nh_quests_init(QuestSystem *qs);

/* Nombre de quêtes dans ce statut. */
int nh_quests_count(const QuestSystem *qs, QuestStatus status);

/* Avancement global en % : quêtes terminées sur QUEST_COUNT. */
int nh_quests_percent(const QuestSystem *qs);

/* Plus haut chapitre entamé (0 si aucune quête n'a démarré). */
int nh_quests_chapter(const QuestSystem *qs);

/* L'objectif `objective` de la quête est accompli. */
bool nh_quest_objective_done(const QuestSystem *qs, QuestType quest, int objective);

/*
 * Relit l'état du jeu : active les quêtes devenues possibles, met l'avancement à jour, termine
 * celles dont tous les objectifs sont accomplis (et enchaîne, jusqu'à ce que plus rien ne
 * change). Annonce ce qui se passe à l'écran. Abonné du bus d'événements.
 */
void nh_quests_refresh(struct GameState *gs);
void nh_quests_on_event(struct GameState *gs, NhEvent event, int value);

/*
 * Termine une quête ACTIVE. `reward` vrai : annonce, verse ses récompenses (crédits, réputation,
 * expérience) et émet NH_EV_QUEST_COMPLETED ; faux : la clôt en silence, sans rien verser (tutoriel
 * passé). Renvoie faux, sans rien faire, si la quête n'est pas active : pas de double récompense.
 */
bool nh_quest_complete(struct GameState *gs, QuestType quest, bool reward);

/* Affichages (le journal complet, le détail d'une quête). */
void nh_quests_print_log(const struct GameState *gs);
void nh_quest_print_details(const struct GameState *gs, QuestType quest);

#endif /* NH_QUEST_SYSTEM_H */
