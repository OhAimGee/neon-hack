#include "i18n.h"

static const char *const k_table[NH_STR_COUNT][NH_LANG_COUNT] = {
#define NH_STR(id, fr, en) [id] = {[NH_LANG_FR] = fr, [NH_LANG_EN] = en},
#include "strings.def"
#undef NH_STR
};

static NhLang g_lang = NH_LANG_EN;

void nh_set_lang(NhLang lang)
{
    if (lang >= 0 && lang < NH_LANG_COUNT)
        g_lang = lang;
}

NhLang nh_get_lang(void) { return g_lang; }

const char *nh_tr_lang(NhStr id, NhLang lang)
{
    if (id < 0 || id >= NH_STR_COUNT || lang < 0 || lang >= NH_LANG_COUNT)
        return "?";
    return k_table[id][lang];
}

const char *nh_tr(NhStr id) { return nh_tr_lang(id, g_lang); }
