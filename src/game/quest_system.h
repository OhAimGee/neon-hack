#ifndef QUEST_SYSTEM_H
#define QUEST_SYSTEM_H

#include "game_types.h"

// Énumérations pour les quêtes
typedef enum
{
    QUEST_INTRO_TUTORIAL,
    QUEST_FIRST_INFILTRATION,
    QUEST_GATHER_INTEL,
    QUEST_NEXUS_DATA_BREACH,
    QUEST_UNDERGROUND_CONTACT,
    QUEST_CORPORATE_SABOTAGE,
    QUEST_AI_LIBERATION,
    QUEST_SHADOW_BROKER,
    QUEST_FINAL_SHOWDOWN,
    QUEST_EPILOGUE,
    QUEST_COUNT
} QuestType;

typedef enum
{
    QUEST_STATUS_LOCKED,
    QUEST_STATUS_AVAILABLE,
    QUEST_STATUS_ACTIVE,
    QUEST_STATUS_COMPLETED,
    QUEST_STATUS_FAILED
} QuestStatus;

typedef enum
{
    OBJECTIVE_HACK_TARGET,
    OBJECTIVE_GATHER_DATA,
    OBJECTIVE_MEET_CONTACT,
    OBJECTIVE_PURCHASE_ITEM,
    OBJECTIVE_REACH_LEVEL,
    OBJECTIVE_DECRYPT_MESSAGE,
    OBJECTIVE_MAINTAIN_STEALTH,
    OBJECTIVE_EARN_CREDITS,
    OBJECTIVE_BUILD_REPUTATION
} ObjectiveType;

// Structures
typedef struct
{
    ObjectiveType type;
    char description[200];
    int target_value;
    int current_value;
    bool is_completed;
    bool is_hidden; // Objectifs secrets
} QuestObjective;

typedef struct
{
    QuestType type;
    char title[100];
    char description[500];
    char lore_text[1000]; // Texte narratif enrichi
    QuestStatus status;
    int level_required;
    int prerequisites[3]; // Quêtes prérequises
    QuestObjective objectives[5];
    int objective_count;

    // Récompenses
    int exp_reward;
    int credits_reward;
    int reputation_reward;
    char special_reward[100]; // Équipement, déblocage, etc.

    // Contexte narratif
    char contact_name[50];
    char location[100];
    bool is_main_quest;
    int chapter;
} Quest;

typedef struct
{
    Quest quests[QUEST_COUNT];
    int active_quest_count;
    int completed_quest_count;
    char current_chapter_title[100];
    int global_story_progress;
} QuestSystem;

// Fonctions
void init_quest_system(QuestSystem *quest_system);
void display_quest_log(const QuestSystem *quest_system);
void display_active_quests(const QuestSystem *quest_system);
void update_quest_progress(QuestSystem *quest_system, ObjectiveType obj_type, int value);
bool start_quest(QuestSystem *quest_system, QuestType quest_type, Player *player);
bool complete_quest(QuestSystem *quest_system, QuestType quest_type, Player *player);
void check_quest_prerequisites(QuestSystem *quest_system, Player *player);
void display_quest_details(const Quest *quest);
void trigger_story_event(QuestSystem *quest_system, const char *event_name);

// Fonctions narratives
void display_chapter_intro(int chapter);
void display_lore_fragment(const char *fragment);
void play_cutscene(const char *cutscene_name);

#endif // QUEST_SYSTEM_H
