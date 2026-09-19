#include "nh_test.h"

#include "nh_capture.h"
#include "nh_feed.h"

#include "../../src/core/platform.h"
#include "../../src/game/commands.h"
#include "../../src/game/contacts.h"
#include "../../src/game/events.h"
#include "../../src/game/progression.h"
#include "../../src/game/quest_system.h"
#include "../../src/game/shop.h"
#include "../../src/game/tutorial.h"
#include "../../src/game/world.h"
#include "../../src/i18n/i18n.h"
#include "../../src/ui/term.h"

#include <stdlib.h>

#define OUT_SIZE 65536

static bool has(const char *text, const char *needle) { return strstr(text, needle) != NULL; }

static int count_of(const char *text, const char *needle)
{
    int n = 0;
    for (const char *p = text; (p = strstr(p, needle)) != NULL; p += strlen(needle))
        n++;
    return n;
}

/* Avant / après dans le texte : vrai si les deux y sont et que `first` vient avant `second`. */
static bool before(const char *text, const char *first, const char *second)
{
    const char *a = strstr(text, first);
    const char *b = strstr(text, second);
    return a != NULL && b != NULL && a < b;
}

static GameState *new_game(void)
{
    GameState *gs = calloc(1, sizeof *gs);
    init_game(gs);
    strcpy(gs->player.name, "Testeur");
    nh_feed(""); /* jamais d'attente sur le vrai clavier */
    return gs;
}

static NhDispatch run(GameState *gs, const char *line, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    NhDispatch r = nh_dispatch(gs, line);
    nh_capture_end(&cap, out, size);
    return r;
}

/* Ce que fait nh_dispatch après une commande : le temps passe, les événements sont livrés. */
static void tick(GameState *gs, char *out, size_t size)
{
    nh_event(gs, NH_EV_COMMAND, NH_DISPATCH_OK);
    NhCapture cap = nh_capture_begin();
    nh_events_flush(gs);
    nh_capture_end(&cap, out, size);
}

static QuestState *state(GameState *gs, QuestType q) { return &gs->quests.quests[q]; }

static void set_level(GameState *gs, int level) { gs->player.level = (HackerLevel)level; }

/* Toutes les quêtes avant `q` sont terminées (sans rien verser), niveau requis atteint : `q` démarre. */
static void start_quest_at(GameState *gs, QuestType q)
{
    char out[OUT_SIZE];
    for (int i = 0; i < (int)q; i++)
    {
        const NhQuestDef *def = nh_quest_def((QuestType)i);
        state(gs, (QuestType)i)->status = QUEST_STATUS_COMPLETED;
        for (int o = 0; o < def->objective_count; o++)
            state(gs, (QuestType)i)->progress[o] = def->objectives[o].target;
    }
    set_level(gs, nh_quest_def(q)->level_required);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, q)->status, QUEST_STATUS_ACTIVE);
}

/* ---- Les tables ------------------------------------------------------------------------------- */

static void check_text(NhStr key)
{
    nh_set_lang(NH_LANG_FR);
    const char *fr = nh_tr(key);
    CHECK(fr != NULL && fr[0] != '\0');
    char fr_copy[2048];
    snprintf(fr_copy, sizeof fr_copy, "%s", fr);
    nh_set_lang(NH_LANG_EN);
    const char *en = nh_tr(key);
    CHECK(en != NULL && en[0] != '\0');
    CHECK(strcmp(fr_copy, en) != 0); /* traduit pour de bon, pas recopié */
    nh_set_lang(NH_LANG_FR);
}

static void test_tables(void)
{
    CHECK(nh_quest_def((QuestType)-1) == NULL);
    CHECK(nh_quest_def(QUEST_COUNT) == NULL);

    int written = 0;
    for (int q = 0; q < QUEST_COUNT; q++)
    {
        const NhQuestDef *def = nh_quest_def((QuestType)q);
        CHECK(def != NULL);
        if (def == NULL)
            continue;
        CHECK(def->objective_count >= 0 && def->objective_count <= NH_QUEST_MAX_OBJECTIVES);
        if (def->objective_count == 0)
            continue; /* pas encore écrite (phase 4) : le reste de la fiche est vide */
        written++;

        check_text(def->title);
        check_text(def->description);
        check_text(def->lore);
        nh_set_lang(NH_LANG_FR);
        CHECK(nh_tr(def->location)[0] != '\0'); /* un nom de lieu peut être identique dans les deux langues */
        CHECK(def->contact != NULL && def->contact[0] != '\0');
        CHECK(def->level_required >= 1 && def->level_required <= NH_LEVEL_MAX);
        CHECK(def->chapter >= 1 && def->chapter <= NH_CHAPTER_COUNT);
        CHECK(def->xp >= 0 && def->credits >= 0 && def->reputation >= 0);

        /* Prérequis : des quêtes écrites, placées avant celle-ci (donc pas de cycle possible). */
        for (int p = 0; p < NH_QUEST_MAX_PREREQ; p++)
        {
            int pre = def->prerequisites[p];
            if (pre < 0)
                continue;
            CHECK(pre < q);
            if (pre < q)
            {
                CHECK(nh_quest_def((QuestType)pre)->objective_count > 0);
                CHECK(nh_quest_def((QuestType)pre)->chapter <= def->chapter);
            }
        }
        if (q > 0)
            CHECK(def->prerequisites[0] >= 0 || def->prerequisites[1] >= 0); /* seul le tutoriel démarre sans */

        for (int o = 0; o < def->objective_count; o++)
        {
            const NhObjectiveDef *obj = &def->objectives[o];
            CHECK(obj->target >= 1);
            check_text(obj->text);

            /* Le seul texte à formater est celui de la condition d'alerte : un %d, pas un de plus. */
            nh_set_lang(NH_LANG_FR);
            int percents = count_of(nh_tr(obj->text), "%");
            CHECK_INT(percents, obj->kind == NH_OBJ_KEEP_ALERT_BELOW ? 1 : 0);

            switch (obj->kind)
            {
            case NH_OBJ_MANUAL:
                CHECK_INT(q, QUEST_INTRO_TUTORIAL); /* rien ne le mesure : seul le tutoriel en a besoin */
                break;
            case NH_OBJ_REACH_LEVEL:
                CHECK(obj->target <= NH_LEVEL_MAX);
                break;
            case NH_OBJ_BUILD_REPUTATION:
                CHECK(obj->target > 1);
                break;
            case NH_OBJ_HACK_TARGET:
                CHECK(obj->arg >= -1 && obj->arg < NH_NODE_COUNT);
                CHECK(obj->target <= (obj->arg < 0 ? NH_NODE_COUNT : 1));
                break;
            case NH_OBJ_GATHER_DATA:
                CHECK(obj->arg >= -1 && obj->arg < NH_NODE_COUNT);
                break;
            case NH_OBJ_MEET_CONTACT:
                CHECK(obj->arg >= 0 && obj->arg < CONTACT_COUNT);
                break;
            case NH_OBJ_PURCHASE_ITEM:
                CHECK(obj->arg > 0 && obj->arg < (1 << ITEM_COUNT));
                break;
            case NH_OBJ_DECRYPT_MESSAGE:
                CHECK(obj->arg >= 0 && obj->arg < NH_MS_COUNT);
                break;
            case NH_OBJ_KEEP_ALERT_BELOW:
                CHECK(obj->arg > 0 && obj->arg <= NH_ALERT_MAX);
                CHECK_INT(obj->target, 1);
                break;
            }
        }
    }
    CHECK_INT(written, 4); /* le tutoriel et les trois premières quêtes ; les autres arrivent en phase 4 */

    CHECK_INT(nh_quest_def(QUEST_INTRO_TUTORIAL)->objectives[0].kind, NH_OBJ_MANUAL);
    CHECK_INT(nh_quest_def(QUEST_INTRO_TUTORIAL)->level_required, 1);
}

/* ---- Pas d'impasse d'expérience ----------------------------------------------------------------- */

static int node_xp(int idx)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    NhCapture cap = nh_capture_begin();
    nh_world_compromise(gs, idx, false);
    nh_capture_end(&cap, out, sizeof out);
    int xp = gs->player.experience;
    free(gs);
    return xp;
}

/*
 * À chaque niveau, ce que le joueur peut encore gagner SANS monter (les scans, les systèmes que le
 * niveau révèle, les quêtes que le niveau ouvre) doit suffire au niveau suivant. Sinon le jeu
 * est bloqué : c'était le cas d'origine au niveau 2 (45 XP disponibles pour 60 requis).
 */
static void test_no_experience_dead_end(void)
{
    int scans = 0;
    for (int i = 0; nh_scan_xp(i) > 0; i++)
        scans += nh_scan_xp(i);

    for (int level = 1; level < NH_LEVEL_MAX; level++)
    {
        int available = scans;
        for (int n = 0; n < NH_NODE_COUNT; n++)
            if (nh_world_min_level(n) <= level)
                available += node_xp(n);
        for (int q = 0; q < QUEST_COUNT; q++)
        {
            const NhQuestDef *def = nh_quest_def((QuestType)q);
            if (def->objective_count > 0 && def->level_required <= level)
                available += def->xp;
        }
        if (available < nh_level_xp_required(level + 1))
            fprintf(stderr, "  niveau %d : %d XP disponibles pour %d requis\n", level, available,
                    nh_level_xp_required(level + 1));
        CHECK(available >= nh_level_xp_required(level + 1));
    }
}

/* ---- État de départ et petits accesseurs ----------------------------------------------------------- */

static void test_initial_state(void)
{
    GameState *gs = new_game();
    CHECK_INT(gs->quests.quests[QUEST_INTRO_TUTORIAL].status, QUEST_STATUS_ACTIVE);
    for (int q = 1; q < QUEST_COUNT; q++)
        CHECK_INT(gs->quests.quests[q].status, QUEST_STATUS_LOCKED);
    CHECK_INT(nh_quests_count(&gs->quests, QUEST_STATUS_ACTIVE), 1);
    CHECK_INT(nh_quests_count(&gs->quests, QUEST_STATUS_LOCKED), QUEST_COUNT - 1);
    CHECK_INT(nh_quests_count(&gs->quests, QUEST_STATUS_COMPLETED), 0);
    CHECK_INT(nh_quests_percent(&gs->quests), 0);
    CHECK_INT(nh_quests_chapter(&gs->quests), 1);

    /* objectifs : rien de fait, et des indices invalides ne sont jamais « faits » */
    CHECK(!nh_quest_objective_done(&gs->quests, QUEST_INTRO_TUTORIAL, 0));
    CHECK(!nh_quest_objective_done(&gs->quests, QUEST_INTRO_TUTORIAL, 1));
    CHECK(!nh_quest_objective_done(&gs->quests, QUEST_INTRO_TUTORIAL, -1));
    CHECK(!nh_quest_objective_done(&gs->quests, (QuestType)-1, 0));
    CHECK(!nh_quest_objective_done(&gs->quests, QUEST_COUNT, 0));

    /* aucune quête démarrée : chapitre 0 ; pourcentage : une quête terminée sur dix = 10 % */
    QuestSystem qs;
    memset(&qs, 0, sizeof qs);
    CHECK_INT(nh_quests_chapter(&qs), 0);
    qs.quests[QUEST_INTRO_TUTORIAL].status = QUEST_STATUS_COMPLETED;
    CHECK_INT(nh_quests_percent(&qs), 10);
    qs.quests[QUEST_FIRST_INFILTRATION].status = QUEST_STATUS_COMPLETED;
    qs.quests[QUEST_GATHER_INTEL].status = QUEST_STATUS_COMPLETED;
    qs.quests[QUEST_NEXUS_DATA_BREACH].status = QUEST_STATUS_COMPLETED;
    CHECK_INT(nh_quests_percent(&qs), 40);
    CHECK_INT(nh_quests_chapter(&qs), 3);
    /* une quête pas encore écrite ne fait pas avancer le chapitre, même si un fichier abîmé la dit active */
    memset(&qs, 0, sizeof qs);
    qs.quests[QUEST_FINAL_SHOWDOWN].status = QUEST_STATUS_ACTIVE;
    CHECK_INT(nh_quests_chapter(&qs), 0);
    free(gs);
}

/* ---- Cycle de vie ---------------------------------------------------------------------------------- */

static void test_unlock_needs_level_and_prerequisite(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];

    /* rien à annoncer, rien ne change : le jeu neuf reste silencieux */
    tick(gs, out, sizeof out);
    CHECK_STR(out, "");
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_LOCKED);

    /* le niveau seul ne suffit pas : le tutoriel n'est pas terminé */
    set_level(gs, 2);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_LOCKED);
    CHECK_STR(out, "");

    /* le tutoriel terminé ne suffit pas non plus au niveau 1 */
    set_level(gs, 1);
    state(gs, QUEST_INTRO_TUTORIAL)->status = QUEST_STATUS_COMPLETED;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_LOCKED);

    /* les deux : la quête démarre, avec annonce */
    nh_set_lang(NH_LANG_FR);
    set_level(gs, 2);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);
    CHECK(has(out, "NOUVELLE QUÊTE : Baptême du Feu"));
    CHECK(has(out, "Tapez 'quests' pour les détails."));

    /* une seule annonce : ensuite plus rien */
    tick(gs, out, sizeof out);
    CHECK_STR(out, "");
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);

    /* la quête suivante attend son niveau et ses prérequis */
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_LOCKED);
    set_level(gs, 3);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_LOCKED); /* Baptême du Feu pas terminée */
    free(gs);
}

static void test_undefined_quests_never_start(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    set_level(gs, NH_LEVEL_MAX);
    for (int q = 0; q < QUEST_COUNT; q++)
        if (nh_quest_def((QuestType)q)->objective_count > 0)
        {
            state(gs, (QuestType)q)->status = QUEST_STATUS_COMPLETED;
            for (int o = 0; o < nh_quest_def((QuestType)q)->objective_count; o++)
                state(gs, (QuestType)q)->progress[o] = nh_quest_def((QuestType)q)->objectives[o].target;
        }

    tick(gs, out, sizeof out);
    CHECK_STR(out, "");
    for (int q = QUEST_UNDERGROUND_CONTACT; q < QUEST_COUNT; q++)
        CHECK_INT(state(gs, (QuestType)q)->status, QUEST_STATUS_LOCKED);

    /* et on ne peut pas les terminer de force */
    CHECK(!nh_quest_complete(gs, QUEST_UNDERGROUND_CONTACT, true));
    CHECK(!nh_quest_complete(gs, QUEST_EPILOGUE, true));
    free(gs);
}

static void test_complete_api(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    int credits0 = gs->player.credits;
    NhCapture cap;

    /* pas active / hors bornes : refusé, rien de versé */
    CHECK(!nh_quest_complete(gs, QUEST_FIRST_INFILTRATION, true));
    CHECK(!nh_quest_complete(gs, (QuestType)-1, true));
    CHECK(!nh_quest_complete(gs, QUEST_COUNT, true));
    CHECK_INT(gs->player.credits, credits0);
    CHECK_INT(nh_events_emitted(gs, NH_EV_QUEST_COMPLETED), 0);

    /* silencieuse : ni récompense ni affichage, mais la quête est bien fermée, objectifs compris */
    cap = nh_capture_begin();
    CHECK(nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, false));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_STR(out, "");
    CHECK_INT(gs->player.credits, credits0);
    CHECK_INT(state(gs, QUEST_INTRO_TUTORIAL)->status, QUEST_STATUS_COMPLETED);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_INTRO_TUTORIAL, 0));
    CHECK_INT(nh_events_emitted(gs, NH_EV_QUEST_COMPLETED), 1);

    /* une quête terminée ne se termine pas deux fois */
    CHECK(!nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, true));
    CHECK_INT(gs->player.credits, credits0);
    CHECK_INT(nh_events_emitted(gs, NH_EV_QUEST_COMPLETED), 1);
    free(gs);

    /* avec récompense : les trois gains et l'annonce, dans la langue courante */
    gs = new_game();
    nh_set_lang(NH_LANG_EN);
    credits0 = gs->player.credits;
    int rep0 = gs->player.reputation;
    cap = nh_capture_begin();
    CHECK(nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, true));
    nh_capture_end(&cap, out, sizeof out);
    CHECK(has(out, "QUEST COMPLETE: First Steps in the Shadows"));
    CHECK_INT(gs->player.credits, credits0 + nh_quest_def(QUEST_INTRO_TUTORIAL)->credits);
    CHECK_INT(gs->player.reputation, rep0 + nh_quest_def(QUEST_INTRO_TUTORIAL)->reputation);
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

/* ---- Les types d'objectifs ------------------------------------------------------------------------ */

static void test_objective_alert_is_a_condition(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_FIRST_INFILTRATION);
    int credits0 = gs->player.credits;

    /* objectif 1 : l'alerte est basse à l'instant même → cochée sans être annoncée */
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_FIRST_INFILTRATION, 1));
    CHECK(!nh_quest_objective_done(&gs->quests, QUEST_FIRST_INFILTRATION, 0));

    /* le système est piraté, mais l'alerte est trop haute : la quête ne se conclut pas */
    gs->nodes[NH_NODE_CORP_SERVER].is_compromised = true;
    gs->alert.level = NH_ALERT_ELEVATED;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_FIRST_INFILTRATION, 0));
    CHECK(!nh_quest_objective_done(&gs->quests, QUEST_FIRST_INFILTRATION, 1)); /* la condition a reculé */
    CHECK(has(out, "Objectif accompli : Infiltrer corp-server-01"));
    CHECK(!has(out, "Maintenir l'alerte")); /* une condition ne s'annonce pas */
    CHECK_INT(gs->player.credits, credits0);

    /* encore un peu plus haut, puis retombée : dès qu'elle repasse sous le seuil, la quête est conclue */
    gs->alert.level = NH_ALERT_ELEVATED + 20;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);
    CHECK(!has(out, "Infiltrer corp-server-01")); /* déjà annoncé : pas de redite */

    gs->alert.level = NH_ALERT_ELEVATED - 1;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_COMPLETED);
    CHECK(has(out, "QUÊTE TERMINÉE : Baptême du Feu"));
    CHECK_INT(gs->player.credits, credits0 + nh_quest_def(QUEST_FIRST_INFILTRATION)->credits);

    /* l'alerte qui remonte ensuite n'y change plus rien */
    gs->alert.level = 90;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_COMPLETED);
    free(gs);
}

static void test_objective_kinds_intel(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_GATHER_INTEL);
    const QuestType Q = QUEST_GATHER_INTEL;
    /* on ne veut pas que la quête se termine en cours de route : la réputation reste hors d'atteinte */

    /* rencontrer un contact : au moins une conversation */
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 0));
    gs->contacts.contacts[CONTACT_ECHO7].interactions_count = 5; /* un autre contact ne compte pas */
    tick(gs, out, sizeof out);
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 0));
    gs->contacts.contacts[CONTACT_R4Z0R].interactions_count = 1;
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 0));
    CHECK(has(out, "Objectif accompli : Contacter R4Z0R"));

    /* acheter un objet précis : un autre achat ne compte pas */
    gs->shop.bought = 1u << ITEM_VIRUS_PACK;
    tick(gs, out, sizeof out);
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 1));
    gs->shop.bought |= 1u << ITEM_STEALTH_UPGRADE;
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 1));

    /* compromettre N systèmes différents : progression partielle visible */
    gs->nodes[NH_NODE_LOCALHOST].is_compromised = true;
    gs->nodes[NH_NODE_CORP_SERVER].is_compromised = true;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->progress[2], 2);
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 2));
    gs->nodes[NH_NODE_MARKET].is_compromised = true;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->progress[2], 3);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 2));

    /* la réputation acquise ne se reperd pas : une fois 50 atteints, l'objectif reste fait */
    gs->player.reputation = 30;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->progress[3], 30);
    gs->player.reputation = -20; /* la perte ne fait pas reculer l'avancement */
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->progress[3], 30);
    gs->player.reputation = 49;
    tick(gs, out, sizeof out);
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 3));
    CHECK_INT(state(gs, Q)->progress[3], 49);
    gs->player.reputation = 50;
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 3));
    CHECK_INT(state(gs, Q)->progress[3], 50);
    gs->player.reputation = 10; /* reperdu ensuite : c'est acquis */
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 3));
    CHECK_INT(state(gs, Q)->progress[3], 50);

    /* quatre sur cinq : la quête n'est pas terminée ; le jalon de décryptage la conclut */
    CHECK_INT(state(gs, Q)->status, QUEST_STATUS_ACTIVE);
    CHECK(nh_milestone_claim(gs, NH_MS_QUANTUM_FIRST)); /* un autre jalon ne compte pas */
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->status, QUEST_STATUS_ACTIVE);
    CHECK(nh_milestone_claim(gs, NH_MS_DECRYPT_TEST));
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->status, QUEST_STATUS_COMPLETED);
    CHECK(has(out, "Objectif accompli : Décrypter la transmission interceptée"));
    CHECK(has(out, "QUÊTE TERMINÉE : Réseaux d'Information"));
    free(gs);
}

static void test_objective_kinds_nexus(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_NEXUS_DATA_BREACH);
    const QuestType Q = QUEST_NEXUS_DATA_BREACH;

    /* un masque d'objets : n'importe lequel suffit, un objet hors masque ne compte pas */
    gs->shop.bought = (1u << ITEM_STEALTH_UPGRADE) | (1u << ITEM_VIRUS_PACK) | (1u << ITEM_REPUTATION_BOOST);
    tick(gs, out, sizeof out);
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 0));
    gs->shop.bought |= 1u << ITEM_AI_MODULE;
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 0));

    /* les fichiers d'un AUTRE système ne comptent pas */
    gs->nodes[NH_NODE_CORP_SERVER].secret_files[0].is_unlocked = true;
    gs->nodes[NH_NODE_CORP_SERVER].is_compromised = true;
    tick(gs, out, sizeof out);
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 1));
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 2));

    /* le système visé compromis, sans extraire */
    gs->nodes[NH_NODE_NEXUS].is_compromised = true;
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, Q, 1));
    CHECK(!nh_quest_objective_done(&gs->quests, Q, 2));
    CHECK_INT(state(gs, Q)->status, QUEST_STATUS_ACTIVE);

    /* un fichier extrait de ce système : la quête est conclue */
    gs->nodes[NH_NODE_NEXUS].secret_files[1].is_unlocked = true;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, Q)->status, QUEST_STATUS_COMPLETED);
    CHECK(has(out, "QUÊTE TERMINÉE : L'œil du Cyclone"));
    free(gs);
}

/* ---- Récompenses ----------------------------------------------------------------------------------- */

static void test_rewards_paid_exactly_once(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_NEXUS_DATA_BREACH);
    const NhQuestDef *def = nh_quest_def(QUEST_NEXUS_DATA_BREACH);
    gs->player.experience = nh_level_xp_required(4);
    int credits0 = gs->player.credits, rep0 = gs->player.reputation, xp0 = gs->player.experience;

    gs->shop.bought = 1u << ITEM_ENCRYPTION_KEY;
    gs->nodes[NH_NODE_NEXUS].is_compromised = true;
    gs->nodes[NH_NODE_NEXUS].secret_files[0].is_unlocked = true;
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_NEXUS_DATA_BREACH)->status, QUEST_STATUS_COMPLETED);
    CHECK_INT(gs->player.credits, credits0 + def->credits);
    CHECK_INT(gs->player.reputation, rep0 + def->reputation);
    CHECK_INT(gs->player.experience, xp0 + def->xp);
    CHECK(has(out, "[+80 EXP]") || has(out, "[+80 XP]"));
    CHECK(has(out, "[+1000 crédits]"));
    CHECK(has(out, "[+100 réputation]"));

    /* encore des événements, encore du temps : plus rien ne se verse */
    for (int i = 0; i < 5; i++)
    {
        nh_event(gs, NH_EV_LEVEL_UP, 5);
        nh_event(gs, NH_EV_NODE_COMPROMISED, NH_NODE_NEXUS);
        tick(gs, out, sizeof out);
        CHECK_STR(out, "");
    }
    CHECK_INT(gs->player.credits, credits0 + def->credits);
    CHECK_INT(gs->player.reputation, rep0 + def->reputation);
    CHECK_INT(gs->player.experience, xp0 + def->xp);
    CHECK_INT(nh_events_emitted(gs, NH_EV_QUEST_COMPLETED), 1);
    free(gs);
}

/* Les événements de la fin d'une quête (niveau gagné) démarrent la suivante dans la même livraison. */
static void test_cascade_level_up_starts_next_quest(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_FIRST_INFILTRATION);
    /* 60 - 25 = 35 XP : la récompense de Baptême du Feu fait passer au niveau 3 */
    gs->player.experience = nh_level_xp_required(3) - nh_quest_def(QUEST_FIRST_INFILTRATION)->xp;
    CHECK_INT(gs->player.level, 2);

    gs->nodes[NH_NODE_CORP_SERVER].is_compromised = true;
    tick(gs, out, sizeof out);

    CHECK_INT(gs->player.level, 3);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_COMPLETED);
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_ACTIVE);
    /* l'ordre à l'écran : objectif, quête terminée, niveau, chapitre 2, nouvelle quête */
    CHECK(before(out, "Objectif accompli", "QUÊTE TERMINÉE : Baptême du Feu"));
    CHECK(before(out, "QUÊTE TERMINÉE : Baptême du Feu", "NIVEAU SUPÉRIEUR"));
    CHECK(before(out, "NIVEAU SUPÉRIEUR", "CHAPITRE 2 : DANS L'OMBRE DES CORPORATIONS"));
    CHECK(before(out, "CHAPITRE 2", "NOUVELLE QUÊTE : Réseaux d'Information"));
    CHECK_INT(count_of(out, "NOUVELLE QUÊTE"), 1);
    free(gs);
}

/* Un joueur qui a déjà tout fait avant que la quête existe (ancienne sauvegarde) la termine d'un coup. */
static void test_old_save_catches_up(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    state(gs, QUEST_INTRO_TUTORIAL)->status = QUEST_STATUS_COMPLETED;
    state(gs, QUEST_FIRST_INFILTRATION)->status = QUEST_STATUS_COMPLETED;
    set_level(gs, 3);
    gs->nodes[NH_NODE_LOCALHOST].is_compromised = true;
    gs->nodes[NH_NODE_CORP_SERVER].is_compromised = true;
    gs->nodes[NH_NODE_MARKET].is_compromised = true;
    gs->shop.bought = 1u << ITEM_STEALTH_UPGRADE;
    gs->contacts.contacts[CONTACT_R4Z0R].interactions_count = 2;
    gs->player.reputation = 80;
    gs->player.milestones = 1u << NH_MS_DECRYPT_TEST;
    int credits0 = gs->player.credits;

    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_COMPLETED);
    CHECK(has(out, "NOUVELLE QUÊTE : Réseaux d'Information"));
    CHECK(has(out, "QUÊTE TERMINÉE : Réseaux d'Information"));
    CHECK(!has(out, "Objectif accompli")); /* ce qui était déjà fait au démarrage n'est pas annoncé */
    CHECK_INT(gs->player.credits, credits0 + nh_quest_def(QUEST_GATHER_INTEL)->credits);
    free(gs);
}

/* ---- Une campagne complète, au niveau de l'API (pas de hasard : les piratages sont posés) ---------------- */

static void test_campaign_first_four_quests(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    NhCapture cap;
    nh_set_lang(NH_LANG_FR);
    nh_set_fast(true);

    /* le tutoriel */
    cap = nh_capture_begin();
    CHECK(nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, true));
    nh_capture_end(&cap, out, sizeof out);
    tick(gs, out, sizeof out);
    CHECK_INT(gs->player.level, 1);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_LOCKED);

    /* niveau 2 par les cinq scans : Baptême du Feu démarre dans la foulée du dernier */
    for (int i = 0; i < 5; i++)
    {
        cap = nh_capture_begin();
        nh_grant_xp(gs, nh_scan_xp(gs->player.scans_done));
        gs->player.scans_done++;
        nh_capture_end(&cap, out, sizeof out);
        tick(gs, out, sizeof out);
    }
    CHECK_INT(gs->player.level, 2);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);

    /* Baptême du Feu : localhost (5 XP, pas dans la quête) puis corp-server-01 */
    cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, NH_NODE_LOCALHOST, false));
    CHECK(nh_world_compromise(gs, NH_NODE_CORP_SERVER, false));
    nh_capture_end(&cap, out, sizeof out);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_COMPLETED);

    /* … et c'est sa récompense qui ouvre le niveau 3, donc Réseaux d'Information */
    CHECK_INT(gs->player.level, 3);
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_ACTIVE);
    CHECK_INT(nh_quests_chapter(&gs->quests), 2);

    /* Réseaux d'Information : contact, achat, troisième système, réputation, décryptage */
    nh_feed("0\n");
    CHECK(run(gs, "contact R4Z0R", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_GATHER_INTEL, 0));
    CHECK_INT(nh_events_pending(gs), 0);

    nh_feed("1\n"); /* Stealth Module */
    CHECK(run(gs, "shop", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_GATHER_INTEL, 1));

    cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, NH_NODE_MARKET, false));
    nh_capture_end(&cap, out, sizeof out);
    tick(gs, out, sizeof out);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_GATHER_INTEL, 2));
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_ACTIVE);

    CHECK_INT(gs->player.reputation, 35); /* tutoriel 10 + Baptême du Feu 25 : 15 de moins que demandé */
    nh_feed("8\n");                       /* Street Cred Booster */
    CHECK(run(gs, "shop", out, sizeof out) == NH_DISPATCH_OK);
    CHECK_INT(gs->player.reputation, 55);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_GATHER_INTEL, 3));
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_ACTIVE);

    cap = nh_capture_begin();
    CHECK(nh_milestone_claim(gs, NH_MS_DECRYPT_TEST));
    nh_capture_end(&cap, out, sizeof out);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_GATHER_INTEL)->status, QUEST_STATUS_COMPLETED);

    /* 15 + 5 + 25 + 25 + 30 + 40 = 140 XP exactement : niveau 4, donc L'œil du Cyclone */
    CHECK_INT(gs->player.level, 4);
    CHECK_INT(state(gs, QUEST_NEXUS_DATA_BREACH)->status, QUEST_STATUS_ACTIVE);
    CHECK_INT(nh_quests_chapter(&gs->quests), 3);
    CHECK(has(out, "CHAPITRE 3 : LE PROJET AURORA"));

    /* L'œil du Cyclone : la clé de chiffrement est abordable, puis Nexus percé en profondeur */
    CHECK(gs->player.credits >= 200);
    nh_feed("5\n"); /* Quantum Encryption Key */
    CHECK(run(gs, "shop", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(nh_quest_objective_done(&gs->quests, QUEST_NEXUS_DATA_BREACH, 0));

    cap = nh_capture_begin();
    CHECK(nh_world_compromise(gs, NH_NODE_NEXUS, true));
    nh_capture_end(&cap, out, sizeof out);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_NEXUS_DATA_BREACH)->status, QUEST_STATUS_COMPLETED);
    CHECK_INT(gs->player.level, 5); /* 140 + 100 + 80 = 320 */

    CHECK_INT(nh_quests_count(&gs->quests, QUEST_STATUS_COMPLETED), 4);
    CHECK_INT(nh_quests_percent(&gs->quests), 40);
    CHECK_INT(nh_quests_count(&gs->quests, QUEST_STATUS_ACTIVE), 0);
    CHECK_INT(nh_events_emitted(gs, NH_EV_QUEST_COMPLETED), 4);

    /* chaque récompense n'a été versée qu'une fois : rejouer des ticks ne change plus rien */
    int credits = gs->player.credits, rep = gs->player.reputation, xp = gs->player.experience;
    for (int i = 0; i < 4; i++)
        tick(gs, out, sizeof out);
    CHECK_INT(gs->player.credits, credits);
    CHECK_INT(gs->player.reputation, rep);
    CHECK_INT(gs->player.experience, xp);
    free(gs);
}

/* ---- Journal et détails ---------------------------------------------------------------------------- */

static void print_log(const GameState *gs, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    nh_quests_print_log(gs);
    nh_capture_end(&cap, out, size);
}

static void print_details(const GameState *gs, QuestType q, char *out, size_t size)
{
    NhCapture cap = nh_capture_begin();
    nh_quest_print_details(gs, q);
    nh_capture_end(&cap, out, size);
}

static void test_log_fr_and_en(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];

    nh_set_lang(NH_LANG_FR);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "=== JOURNAL DE QUÊTES ==="));
    CHECK(has(out, "=== QUÊTES ACTIVES ==="));
    CHECK(has(out, "[1] Premiers Pas dans l'Ombre"));
    CHECK(has(out, "Contact: ECHO-7"));
    CHECK(has(out, "Aucune quête terminée."));
    CHECK(has(out, "Progression globale : 0 %"));
    CHECK(has(out, "CHAPITRE 1 : L'ÉVEIL DU HACKER"));

    nh_set_lang(NH_LANG_EN);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "=== QUEST LOG ==="));
    CHECK(has(out, "=== ACTIVE QUESTS ==="));
    CHECK(has(out, "[1] First Steps in the Shadows"));
    CHECK(has(out, "No completed quest."));
    CHECK(has(out, "Overall progress: 0%"));
    CHECK(has(out, "CHAPTER 1: THE HACKER'S AWAKENING"));
    /* pas de français résiduel dans le journal anglais */
    CHECK(!has(out, "QUÊTES"));
    CHECK(!has(out, "Objectifs"));
    CHECK(!has(out, "Lieu"));
    CHECK(!has(out, "Aucune"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_log_next_quest_hint(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);

    /* tutoriel terminé, niveau 1 : la suite n'attend que le niveau, le journal le dit */
    state(gs, QUEST_INTRO_TUTORIAL)->status = QUEST_STATUS_COMPLETED;
    print_log(gs, out, sizeof out);
    CHECK(has(out, "Aucune quête active."));
    CHECK(has(out, "Prochaine quête : atteignez le niveau 2."));
    CHECK(has(out, "[1] Premiers Pas dans l'Ombre")); /* dans la liste des terminées */
    CHECK(has(out, "Progression globale : 10 %"));

    nh_set_lang(NH_LANG_EN);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "Next quest: reach level 2."));
    nh_set_lang(NH_LANG_FR);

    /* quand le niveau est atteint mais que la quête n'a pas démarré (pas de livraison encore), pas d'indice faux */
    set_level(gs, 2);
    print_log(gs, out, sizeof out);
    CHECK(!has(out, "Prochaine quête"));

    /* toutes les quêtes écrites sont terminées, les suivantes n'existent pas encore : pas d'indice non plus */
    for (int q = 0; q < QUEST_COUNT; q++)
        if (nh_quest_def((QuestType)q)->objective_count > 0)
            state(gs, (QuestType)q)->status = QUEST_STATUS_COMPLETED;
    print_log(gs, out, sizeof out);
    CHECK(!has(out, "Prochaine quête"));
    CHECK(has(out, "Progression globale : 40 %"));
    free(gs);
}

static void test_log_objectives_and_secret(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_GATHER_INTEL);

    print_log(gs, out, sizeof out);
    CHECK(has(out, "[3] Réseaux d'Information"));
    CHECK(has(out, "[ ] Compromettre 3 systèmes différents (0/3)"));
    CHECK(has(out, "[ ] Atteindre 50 points de réputation"));
    CHECK(has(out, "(0/50)"));
    /* l'objectif secret ne dit rien de lui-même tant qu'il n'est pas accompli */
    CHECK(has(out, "[ ] ??? (objectif secret)"));
    CHECK(!has(out, "Décrypter la transmission"));

    gs->nodes[NH_NODE_LOCALHOST].is_compromised = true;
    gs->nodes[NH_NODE_CORP_SERVER].is_compromised = true;
    gs->player.milestones |= 1u << NH_MS_DECRYPT_TEST;
    char scratch[OUT_SIZE];
    tick(gs, scratch, sizeof scratch);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "[ ] Compromettre 3 systèmes différents (2/3)"));
    CHECK(has(out, "[x] Décrypter la transmission interceptée (decrypt)"));
    CHECK(!has(out, "??? (objectif secret)"));

    nh_set_lang(NH_LANG_EN);
    gs->player.milestones = 0;
    state(gs, QUEST_GATHER_INTEL)->progress[4] = 0;
    print_log(gs, out, sizeof out);
    CHECK(has(out, "[ ] ??? (secret objective)"));
    CHECK(has(out, "[ ] Compromise 3 different systems (2/3)"));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

static void test_log_alert_number_in_text(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    start_quest_at(gs, QUEST_FIRST_INFILTRATION);

    nh_set_lang(NH_LANG_FR);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "[x] Maintenir l'alerte sous 50 (laylow la fait baisser)"));
    CHECK(!has(out, "%d"));
    nh_set_lang(NH_LANG_EN);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "[x] Keep your alert below 50 (laylow lowers it)"));
    nh_set_lang(NH_LANG_FR);

    gs->alert.level = 70;
    char scratch[OUT_SIZE];
    tick(gs, scratch, sizeof scratch);
    print_log(gs, out, sizeof out);
    CHECK(has(out, "[ ] Maintenir l'alerte sous 50"));
    free(gs);
}

static void test_details(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    start_quest_at(gs, QUEST_FIRST_INFILTRATION);

    nh_set_lang(NH_LANG_FR);
    print_details(gs, QUEST_FIRST_INFILTRATION, out, sizeof out);
    CHECK(has(out, "=== DÉTAILS DE QUÊTE ==="));
    CHECK(has(out, "Baptême du Feu"));
    CHECK(has(out, "Contexte :"));
    CHECK(has(out, "Objectifs :"));
    CHECK(has(out, "Récompenses :"));
    CHECK(has(out, "25 XP"));
    CHECK(has(out, "200 crédits"));
    CHECK(has(out, "25 réputation"));

    nh_set_lang(NH_LANG_EN);
    print_details(gs, QUEST_FIRST_INFILTRATION, out, sizeof out);
    CHECK(has(out, "=== QUEST DETAILS ==="));
    CHECK(has(out, "Baptism of Fire"));
    CHECK(has(out, "Background:"));
    CHECK(has(out, "Rewards:"));
    CHECK(has(out, "200 credits"));
    CHECK(!has(out, "Contexte"));
    nh_set_lang(NH_LANG_FR);

    /* une quête terminée a déjà payé : le détail ne promet plus rien */
    state(gs, QUEST_FIRST_INFILTRATION)->status = QUEST_STATUS_COMPLETED;
    print_details(gs, QUEST_FIRST_INFILTRATION, out, sizeof out);
    CHECK(has(out, "Baptême du Feu"));
    CHECK(!has(out, "Récompenses"));

    /* hors bornes ou pas écrite : rien du tout */
    print_details(gs, QUEST_EPILOGUE, out, sizeof out);
    CHECK_STR(out, "");
    print_details(gs, (QuestType)-1, out, sizeof out);
    CHECK_STR(out, "");
    print_details(gs, QUEST_COUNT, out, sizeof out);
    CHECK_STR(out, "");
    free(gs);
}

/* ---- La commande `quests` --------------------------------------------------------------------------- */

static void test_cmd_quests(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    start_quest_at(gs, QUEST_FIRST_INFILTRATION);

    /* 0 : le journal seul */
    nh_feed("0\n");
    CHECK(run(gs, "quests", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "=== JOURNAL DE QUÊTES ==="));
    CHECK(has(out, "Détails d'une quête ?"));
    CHECK(!has(out, "DÉTAILS DE QUÊTE"));

    /* le numéro d'une quête active ou terminée : son détail */
    nh_feed("2\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "=== DÉTAILS DE QUÊTE ==="));
    CHECK(has(out, "Baptême du Feu"));
    nh_feed("1\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "=== DÉTAILS DE QUÊTE ==="));
    CHECK(has(out, "Premiers Pas dans l'Ombre"));

    /* verrouillée, pas écrite ou hors du journal : « aucune quête n° » */
    nh_feed("3\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "Aucune quête n° 3 dans votre journal."));
    CHECK(!has(out, "DÉTAILS DE QUÊTE"));
    nh_feed("7\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "Aucune quête n° 7 dans votre journal."));
    nh_feed("99\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "Aucune quête n° 99 dans votre journal."));

    /* n'importe quoi : option invalide, jamais de plantage */
    const char *junk[] = {"abc\n", "-1\n", "100\n", "1x\n", "  \n"};
    for (size_t i = 0; i < sizeof junk / sizeof junk[0]; i++)
    {
        nh_feed(junk[i]);
        CHECK(run(gs, "quests", out, sizeof out) == NH_DISPATCH_OK);
        CHECK(!has(out, "DÉTAILS DE QUÊTE"));
    }
    nh_feed("abc\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "Option invalide."));

    /* Entrée seule ou fin d'entrée : on sort sans rien dire de plus */
    nh_feed("\n");
    CHECK(run(gs, "quests", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(!has(out, "Option invalide"));
    nh_feed("");
    CHECK(run(gs, "quests", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(!has(out, "Option invalide"));
    CHECK(gs->running);

    /* en anglais, le même dialogue */
    nh_set_lang(NH_LANG_EN);
    nh_feed("2\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "Quest details? (its number, or 0 to leave)"));
    CHECK(has(out, "=== QUEST DETAILS ==="));
    nh_feed("7\n");
    run(gs, "quests", out, sizeof out);
    CHECK(has(out, "No quest #7 in your log."));
    nh_set_lang(NH_LANG_FR);
    free(gs);
}

/* Pendant le tutoriel, `quests` montre la mission d'ECHO-7 : pas de question, pas de journal. */
static void test_cmd_quests_during_tutorial(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    NhCapture cap = nh_capture_begin();
    nh_tutorial_start(gs);
    nh_capture_end(&cap, out, sizeof out);

    nh_feed("1\n");
    CHECK(run(gs, "quests", out, sizeof out) == NH_DISPATCH_OK);
    CHECK(has(out, "MISSION EN COURS"));
    CHECK(!has(out, "Détails d'une quête"));
    free(gs);
}

/* ---- Quêtes et événements dans la boucle de jeu --------------------------------------------------- */

/* L'annonce d'une quête vient APRÈS le résultat de la commande qui la déclenche, pas au milieu. */
static void test_announcement_follows_command_output(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    state(gs, QUEST_INTRO_TUTORIAL)->status = QUEST_STATUS_COMPLETED;
    gs->player.experience = 10; /* à un scan du niveau 2 : 5 + 4 + 3 + 2 + 1 = 15 */
    gs->player.scans_done = 1;

    for (int i = 0; i < 3; i++)
    {
        run(gs, "scan", out, sizeof out);
        if (gs->player.level == LEVEL_APPRENTICE)
            break;
    }
    CHECK_INT(gs->player.level, LEVEL_APPRENTICE);
    CHECK(has(out, "NOUVELLE QUÊTE : Baptême du Feu"));
    CHECK(before(out, "NIVEAU SUPÉRIEUR", "NOUVELLE QUÊTE"));
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);
    CHECK_INT(nh_events_pending(gs), 0);
    free(gs);
}

/* Le tutoriel terminé avec récompense ouvre la première vraie quête si le niveau est déjà bon. */
static void test_tutorial_reward_and_chain(void)
{
    GameState *gs = new_game();
    char out[OUT_SIZE];
    nh_set_lang(NH_LANG_FR);
    set_level(gs, 2);

    NhCapture cap = nh_capture_begin();
    CHECK(nh_quest_complete(gs, QUEST_INTRO_TUTORIAL, true));
    nh_capture_end(&cap, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_LOCKED); /* pas avant la livraison */
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);
    CHECK(has(out, "NOUVELLE QUÊTE : Baptême du Feu"));
    free(gs);

    /* passé en silence, le tutoriel débloque quand même la suite */
    gs = new_game();
    set_level(gs, 2);
    cap = nh_capture_begin();
    nh_tutorial_start(gs);
    nh_tutorial_skip(gs);
    nh_capture_end(&cap, out, sizeof out);
    tick(gs, out, sizeof out);
    CHECK_INT(state(gs, QUEST_FIRST_INFILTRATION)->status, QUEST_STATUS_ACTIVE);
    free(gs);
}

int main(void)
{
    nh_set_fast(true);
    nh_term_set_color(false);
    nh_set_lang(NH_LANG_FR);

    test_tables();
    test_no_experience_dead_end();
    test_initial_state();
    test_unlock_needs_level_and_prerequisite();
    test_undefined_quests_never_start();
    test_complete_api();
    test_objective_alert_is_a_condition();
    test_objective_kinds_intel();
    test_objective_kinds_nexus();
    test_rewards_paid_exactly_once();
    test_cascade_level_up_starts_next_quest();
    test_old_save_catches_up();
    test_campaign_first_four_quests();
    test_log_fr_and_en();
    test_log_next_quest_hint();
    test_log_objectives_and_secret();
    test_log_alert_number_in_text();
    test_details();
    test_cmd_quests();
    test_cmd_quests_during_tutorial();
    test_announcement_follows_command_output();
    test_tutorial_reward_and_chain();
    nh_unfeed();
    return NH_TEST_REPORT("quests");
}
