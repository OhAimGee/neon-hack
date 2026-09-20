#ifndef CONTACTS_H
#define CONTACTS_H

/*
 * Les contacts et la boîte de réception.
 *
 * Comme les quêtes et la boutique, tout ce qui est du CONTENU (noms, fiches, répliques, courriers)
 * est dans des tables `static const` de contacts.c dont les textes sont des clés de strings.def :
 * ils suivent la langue et ne sont jamais sauvegardés. L'état ne garde que ce qui change en cours
 * de partie : qui est débloqué, combien de conversations, quels courriers sont arrivés et lus.
 *
 * Déblocage : un contact apparaît quand le niveau, la réputation ET (s'il y en a une) la quête
 * demandés par sa fiche sont atteints. C'est un abonné du bus d'événements (nh_contacts_on_event),
 * comme le moteur de quêtes : il relit l'état du jeu, il ne compte rien. Les cinq derniers contacts
 * (Shadow Broker, Neon Angel, Ghost Walker, Data Miner, Nexus Insider) n'ont pas encore de fiche
 * (phase 4.2) : ils ont un nom, un emplacement, et restent verrouillés.
 *
 * Conversation : un menu de sujets propre au contact, qui se répète jusqu'à « Terminer », 0 ou la fin
 * de l'entrée. Un sujet est une simple réplique, le commentaire du niveau du joueur, la mission en
 * cours de ce contact, l'ouverture de la boutique, ou son dossier. Rien n'est promis qui ne soit
 * tenu : la liste des services d'un contact se déduit de ses sujets.
 *
 * Courriers : chaque courrier est un modèle de la table (expéditeur, sujet, texte) ; la boîte de
 * réception retient seulement quels modèles sont arrivés, dans l'ordre, et lesquels sont lus. Un
 * modèle n'arrive qu'une fois. Les courriers de présentation partent au déblocage du contact.
 */

#include <stdbool.h>

#include "events.h"

struct GameState;

/* Les valeurs sont écrites dans les sauvegardes (contact.N.*) : ne pas les réordonner. */
typedef enum
{
    CONTACT_ECHO7,         // Mentor initial
    CONTACT_R4Z0R,         // Vendeur du marché noir
    CONTACT_PHOENIX,       // Hacker expert anonyme
    CONTACT_AURA,          // IA libérée
    CONTACT_SHADOW_BROKER, // Courtier d'informations (fiche à écrire : phase 4.2)
    CONTACT_NEON_ANGEL,    // Activiste cyber (phase 4.2)
    CONTACT_GHOST_WALKER,  // Spécialiste infiltration (phase 4.2)
    CONTACT_DATA_MINER,    // Expert extraction de données (phase 4.2)
    CONTACT_NEXUS_INSIDER, // Informateur corporate (phase 4.2)
    CONTACT_COUNT
} ContactType;

typedef enum
{
    NH_REL_UNKNOWN,
    NH_REL_NEUTRAL,
    NH_REL_FRIENDLY,
    NH_REL_TRUSTED
} NhRelation;

/* Les modèles de courrier. Les valeurs sont écrites dans les sauvegardes (mail.N.id) : ne pas les réordonner. */
typedef enum
{
    NH_MAIL_WELCOME, // ECHO-7, dès le départ
    NH_MAIL_R4Z0R,   // à la rencontre de R4Z0R
    NH_MAIL_PHOENIX,
    NH_MAIL_AURA,
    NH_MAIL_COUNT
} NhMail;

typedef struct
{
    bool is_unlocked;
    int interactions_count; // conversations tenues
} Contact;

typedef struct
{
    NhMail mail;
    bool is_read;
} Mail;

#define NH_INBOX_MAX 32 /* > NH_MAIL_COUNT : un modèle n'arrive qu'une fois */

typedef struct
{
    Contact contacts[CONTACT_COUNT];
    Mail inbox[NH_INBOX_MAX]; // dans l'ordre d'arrivée
    int inbox_count;
} ContactSystem;

/* État initial : ECHO-7 est joignable et son courrier de bienvenue attend, non lu. Aucune sortie. */
void init_contact_system(ContactSystem *contact_system);

/* Nom propre (jamais traduit) ; "?" hors bornes. */
const char *nh_contact_name(ContactType type);

/* La fiche et les sujets de ce contact sont écrits (sinon il ne se débloque jamais). */
bool nh_contact_written(ContactType type);

/*
 * Confiance 0..100, DÉDUITE : la confiance de départ de sa fiche, plus 3 points par conversation.
 * Elle ne sert qu'à l'affichage (relation) tant que les contacts n'ont pas de service à prix de
 * confiance (phase 4.2).
 */
int nh_contact_trust(const ContactSystem *contact_system, ContactType type);
NhRelation nh_contact_relation(int trust);

/* Contacts débloqués. */
int nh_contacts_unlocked(const ContactSystem *contact_system);

/*
 * Retrouve un contact débloqué : `text` est son numéro dans la liste (1 = le premier débloqué, dans
 * l'ordre de la table) ou son nom, sans tenir compte de la casse. Renvoie -1 s'il n'y en a pas.
 */
int nh_contact_find(const ContactSystem *contact_system, const char *text);

/*
 * Abonné du bus d'événements : débloque, avec annonce et courrier de présentation, les contacts
 * dont les conditions sont remplies. Un contact débloqué le reste.
 */
void nh_contacts_on_event(struct GameState *gs, NhEvent event, int value);

/*
 * Conversation avec un contact débloqué (menu de sujets, sorties incluses). Compte la conversation,
 * émet NH_EV_CONTACT_MET, et renvoie faux sans rien afficher si le contact n'est pas joignable.
 */
bool nh_contact_talk(struct GameState *gs, ContactType type);

/* Dépose un courrier dans la boîte de réception et l'annonce ; faux s'il y est déjà (ou boîte pleine). */
bool nh_mail_send(struct GameState *gs, NhMail mail);

int nh_inbox_unread(const ContactSystem *contact_system);

/* Affichages (`contacts` et `messages`). */
void nh_contacts_print(const struct GameState *gs);
void nh_inbox_print(const struct GameState *gs);

#endif // CONTACTS_H
