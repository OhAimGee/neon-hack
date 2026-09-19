#include "intro.h"

#include <stdio.h>
#include <string.h>

#include "../core/io.h"
#include "../core/parse.h"
#include "../core/platform.h"
#include "../i18n/i18n.h"
#include "../ui/term.h"
#include "tutorial.h"

/* Vitesses de l'effet machine à écrire (ms par caractère) : le récit est plus lent que les répliques. */
#define NARRATION_DELAY 14
#define ECHO_DELAY 9

static void narrate(NhStr text)
{
    char wrapped[1024];
    nh_wrap_text(wrapped, sizeof wrapped, nh_tr(text), 0, 0, nh_wrap_width());
    nh_typewriter(wrapped, NARRATION_DELAY);
    printf("\n");
    nh_sleep_ms(250);
}

static void echo(NhStr text)
{
    nh_echo_say(nh_tr(text), true, ECHO_DELAY);
    nh_sleep_ms(200);
}

/* Pose la question du nom jusqu'à ce que le joueur confirme. Faux si l'entrée est fermée. */
static bool ask_name(char *name, size_t size)
{
    char line[256];

    for (;;)
    {
        printf("%s", nh_c(NH_C_YELLOW));
        printf(nh_tr(NH_STR_INTRO_NAME_PROMPT), NH_DEFAULT_NAME);
        printf("%s", nh_c(NH_C_RESET));
        fflush(stdout);
        if (nh_read_line(line, sizeof line) != NH_IO_OK)
            return false;

        /* Rien de présentable (vide, espaces, caractères de contrôle) : le nom par défaut. */
        if (nh_clean_name(line, name, size, NH_NAME_MAX_CHARS) == 0)
            snprintf(name, size, "%s", NH_DEFAULT_NAME);

        char question[256];
        snprintf(question, sizeof question, nh_tr(NH_STR_INTRO_ECHO_CONFIRM), name);
        nh_echo_say(question, false, 0);
        fflush(stdout);
        if (nh_read_line(line, sizeof line) != NH_IO_OK)
            return false;
        if (nh_parse_yes_no(line, true))
            return true;

        printf("\n");
        echo(NH_STR_INTRO_ECHO_RETRY);
    }
}

/* 1 = tutoriel, 2 = se débrouiller. 0 si l'entrée est fermée. */
static int ask_tutorial(void)
{
    char line[64];

    printf("  %s[1]%s %s\n", nh_c(NH_C_YELLOW), nh_c(NH_C_RESET), nh_tr(NH_STR_INTRO_CHOICE_TUTORIAL));
    printf("  %s[2]%s %s\n", nh_c(NH_C_YELLOW), nh_c(NH_C_RESET), nh_tr(NH_STR_INTRO_CHOICE_SKIP));

    for (;;)
    {
        printf("%s%s%s", nh_c(NH_C_YELLOW), nh_tr(NH_STR_INTRO_CHOICE_PROMPT), nh_c(NH_C_RESET));
        fflush(stdout);
        if (nh_read_line(line, sizeof line) != NH_IO_OK)
            return 0;

        int choice = 1; /* Entrée seule : le tutoriel */
        if (line[0] != '\0' && !nh_parse_int(line, 1, 2, &choice))
        {
            printf("%s\n", nh_tr(NH_STR_INVALID_OPTION));
            continue;
        }
        return choice;
    }
}

NhIntroResult nh_intro_run(GameState *gs)
{
    printf("\n%s=== %s ===%s\n\n", nh_c(NH_C_MAGENTA), nh_tr(NH_STR_INTRO_TITLE), nh_c(NH_C_RESET));
    narrate(NH_STR_INTRO_NARR_1);
    narrate(NH_STR_INTRO_NARR_2);
    narrate(NH_STR_INTRO_NARR_3);
    printf("\n");
    echo(NH_STR_INTRO_ECHO_1);
    echo(NH_STR_INTRO_ECHO_2);
    printf("\n");

    char name[MAX_NAME_LENGTH];
    if (!ask_name(name, sizeof name))
        return NH_INTRO_EOF;
    snprintf(gs->player.name, sizeof gs->player.name, "%s", name);

    printf("\n");
    char named[256];
    snprintf(named, sizeof named, nh_tr(NH_STR_INTRO_ECHO_NAMED), gs->player.name);
    nh_echo_say(named, true, ECHO_DELAY);
    nh_sleep_ms(200);
    echo(NH_STR_INTRO_ECHO_OFFER);

    int choice = ask_tutorial();
    if (choice == 0)
        return NH_INTRO_EOF;

    printf("\n");
    if (choice == 1)
    {
        nh_tutorial_start(gs);
        echo(NH_STR_INTRO_ECHO_TUTORIAL);
    }
    else
    {
        nh_tutorial_skip(gs);
        echo(NH_STR_INTRO_ECHO_SKIP);
        printf("\n%s%s%s\n", nh_c(NH_C_CYAN), nh_tr(NH_STR_HINT_HELP), nh_c(NH_C_RESET));
    }
    printf("\n");
    return NH_INTRO_OK;
}
