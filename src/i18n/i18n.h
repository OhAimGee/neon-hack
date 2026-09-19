#ifndef NH_I18N_H
#define NH_I18N_H

#include "../core/config.h"

typedef enum
{
#define NH_STR(id, fr, en) id,
#include "strings.def"
#undef NH_STR
    NH_STR_COUNT
} NhStr;

void nh_set_lang(NhLang lang);
NhLang nh_get_lang(void);

/* Texte dans la langue courante (jamais NULL). Peut servir de format printf. */
const char *nh_tr(NhStr id);

/* Texte dans une langue précise (utile pour les tests). */
const char *nh_tr_lang(NhStr id, NhLang lang);

#endif /* NH_I18N_H */
