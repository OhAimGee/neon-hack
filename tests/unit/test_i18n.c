#include "nh_test.h"

#include "../../src/i18n/i18n.h"

#include <ctype.h>

/*
 * Signature printf d'un texte : la suite des spécificateurs de conversion
 * ("%s%d" pour "Bonjour %s, niveau %d"). "%%" est ignoré.
 */
static void format_signature(const char *text, char *sig, size_t size)
{
    size_t n = 0;
    for (const char *p = text; *p; p++)
    {
        if (*p != '%')
            continue;
        p++;
        if (*p == '%')
            continue;
        while (*p && !isalpha((unsigned char)*p))
            p++;
        while (*p == 'l' || *p == 'h' || *p == 'z' || *p == 'j' || *p == 't')
            p++;
        if (*p && n + 2 < size)
        {
            sig[n++] = '%';
            sig[n++] = *p;
        }
        if (*p == '\0')
            break;
    }
    sig[n] = '\0';
}

static void test_signature_helper(void)
{
    char sig[32];
    format_signature("Bonjour %s, niveau %d !", sig, sizeof sig);
    CHECK_STR(sig, "%s%d");
    format_signature("100%% sûr", sig, sizeof sig);
    CHECK_STR(sig, "");
    format_signature("%-8s|%5ld", sig, sizeof sig);
    CHECK_STR(sig, "%s%d");
}

static void test_all_strings(void)
{
    for (int id = 0; id < NH_STR_COUNT; id++)
    {
        const char *fr = nh_tr_lang((NhStr)id, NH_LANG_FR);
        const char *en = nh_tr_lang((NhStr)id, NH_LANG_EN);

        CHECK(fr != NULL && fr[0] != '\0');
        CHECK(en != NULL && en[0] != '\0');
        if (fr == NULL || en == NULL)
            continue;

        char sig_fr[64], sig_en[64];
        format_signature(fr, sig_fr, sizeof sig_fr);
        format_signature(en, sig_en, sizeof sig_en);
        CHECK_STR(sig_fr, sig_en); /* même arguments printf dans les deux langues */
        if (strcmp(sig_fr, sig_en) != 0)
            fprintf(stderr, "    clé %d : \"%s\" / \"%s\"\n", id, fr, en);
    }
}

static void test_language_switch(void)
{
    nh_set_lang(NH_LANG_FR);
    CHECK_INT(nh_get_lang(), NH_LANG_FR);
    CHECK_STR(nh_tr(NH_STR_INVALID_OPTION), "Option invalide.");

    nh_set_lang(NH_LANG_EN);
    CHECK_INT(nh_get_lang(), NH_LANG_EN);
    CHECK_STR(nh_tr(NH_STR_INVALID_OPTION), "Invalid option.");

    nh_set_lang((NhLang)99); /* ignoré */
    CHECK_INT(nh_get_lang(), NH_LANG_EN);

    CHECK_STR(nh_tr_lang((NhStr)9999, NH_LANG_FR), "?");
    CHECK_STR(nh_tr_lang((NhStr)-1, NH_LANG_FR), "?");
    CHECK_STR(nh_tr_lang(NH_STR_BYE_1, (NhLang)7), "?");
}

int main(void)
{
    test_signature_helper();
    test_all_strings();
    test_language_switch();
    return NH_TEST_REPORT("i18n");
}
