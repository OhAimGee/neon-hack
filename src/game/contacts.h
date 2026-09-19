#ifndef CONTACTS_H
#define CONTACTS_H

#include "game_types.h"

// Énumérations pour les contacts
typedef enum
{
    CONTACT_ECHO7,         // Mentor initial
    CONTACT_R4Z0R,         // Vendeur du marché noir
    CONTACT_PHOENIX,       // Hacker expert anonyme
    CONTACT_AURA,          // IA libérée
    CONTACT_SHADOW_BROKER, // Courtier d'informations
    CONTACT_NEON_ANGEL,    // Activiste cyber
    CONTACT_GHOST_WALKER,  // Spécialiste infiltration
    CONTACT_DATA_MINER,    // Expert extraction de données
    CONTACT_NEXUS_INSIDER, // Informateur corporate
    CONTACT_COUNT
} ContactType;

typedef enum
{
    RELATION_UNKNOWN,
    RELATION_NEUTRAL,
    RELATION_FRIENDLY,
    RELATION_ALLIED,
    RELATION_HOSTILE,
    RELATION_TRUSTED
} RelationStatus;

typedef enum
{
    CONTACT_AVAILABLE,
    CONTACT_BUSY,
    CONTACT_OFFLINE,
    CONTACT_COMPROMISED
} ContactAvailability;

// Structures
typedef struct
{
    ContactType type;
    char name[50];
    char real_name[50];
    char description[200];
    char speciality[100];
    char location[100];
    RelationStatus relation;
    ContactAvailability availability;

    int trust_level; // 0-100
    int reputation_required;
    int level_required;
    bool is_unlocked;
    bool is_discovered;

    // Services offerts
    bool can_sell_items;
    bool can_give_missions;
    bool can_provide_intel;
    bool can_decode_messages;
    bool can_hack_assistance;

    // Historique d'interaction
    int interactions_count;
    int missions_completed;
    int last_contact_time;

    // Dialogue et personnalité
    char greeting[200];
    char personality_trait[100];
    char backstory[500];
} Contact;

typedef struct
{
    char from[50];
    char to[50];
    char subject[100];
    char content[1000];
    bool is_encrypted;
    bool is_read;
    time_t timestamp;
    int priority; // 1-5, 5 étant urgent
} Message;

typedef struct
{
    Contact contacts[CONTACT_COUNT];
    Message inbox[50];
    Message sent_messages[50];
    int inbox_count;
    int sent_count;
    int active_contacts;
} ContactSystem;

// Fonctions principales
void init_contact_system(ContactSystem *contact_system);
void display_contacts(const ContactSystem *contact_system);
void display_contact_details(const Contact *contact);
bool contact_npc(ContactSystem *contact_system, ContactType contact_type, Player *player);
void send_message(ContactSystem *contact_system, const char *to, const char *subject, const char *content);
void check_messages(ContactSystem *contact_system);
void display_inbox(const ContactSystem *contact_system);

// Fonctions de dialogue
void start_dialogue(ContactSystem *contact_system, ContactType contact_type, Player *player);
void display_dialogue_options(ContactType contact_type, Player *player);
void process_dialogue_choice(ContactSystem *contact_system, ContactType contact_type, int choice, Player *player);

// Fonctions d'interaction
bool unlock_contact(ContactSystem *contact_system, ContactType contact_type, Player *player);
void improve_relation(ContactSystem *contact_system, ContactType contact_type, int amount);
void update_contact_availability(ContactSystem *contact_system);

// Fonctions spécialisées
void echo7_interaction(ContactSystem *contact_system, Player *player);
void r4z0r_interaction(ContactSystem *contact_system, Player *player);
void phoenix_interaction(ContactSystem *contact_system, Player *player);
void aura_interaction(ContactSystem *contact_system, Player *player);

#endif // CONTACTS_H
