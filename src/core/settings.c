#include "settings.h"

#include <stdlib.h>
#include <string.h>

#include "kv.h"
#include "storage.h"

#define SETTINGS_MAX_BYTES 4096

static const char *const k_lang_codes[NH_LANG_COUNT] = {[NH_LANG_FR] = "fr", [NH_LANG_EN] = "en"};

void nh_settings_from_text(NhSettings *s, const char *text, size_t len)
{
    NhKv kv;
    if (!nh_kv_parse(&kv, text, len))
        return;

    const char *lang = nh_kv_get(&kv, "lang");
    if (lang != NULL)
        for (int i = 0; i < NH_LANG_COUNT; i++)
            if (strcmp(lang, k_lang_codes[i]) == 0)
                s->lang = (NhLang)i;

    long value;
    if (nh_kv_int(&kv, "color", 0, 1, &value) == NH_KV_OK)
        s->color = value != 0;
    if (nh_kv_int(&kv, "fast", 0, 1, &value) == NH_KV_OK)
        s->fast = value != 0;
    if (nh_kv_int(&kv, "hud", 0, 1, &value) == NH_KV_OK)
        s->hud = value != 0;

    nh_kv_free(&kv);
}

void nh_settings_to_text(const NhSettings *s, char *out, size_t size)
{
    NhKvWriter w;
    nh_kvw_init(&w);
    nh_kvw_str(&w, "lang", k_lang_codes[s->lang >= 0 && s->lang < NH_LANG_COUNT ? s->lang : NH_LANG_EN]);
    nh_kvw_int(&w, "color", s->color);
    nh_kvw_int(&w, "fast", s->fast);
    nh_kvw_int(&w, "hud", s->hud);

    if (size > 0)
        snprintf(out, size, "%s", nh_kvw_text(&w));
    nh_kvw_free(&w);
}

bool nh_settings_load(NhSettings *s, const char *path)
{
    char *data = NULL;
    size_t len = 0;
    if (nh_storage_read(path, SETTINGS_MAX_BYTES, &data, &len) != NH_STORAGE_OK)
        return false;
    nh_settings_from_text(s, data, len);
    free(data);
    return true;
}

bool nh_settings_save(const NhSettings *s, const char *path)
{
    char text[256];
    nh_settings_to_text(s, text, sizeof text);
    return nh_storage_write_atomic(path, text, strlen(text));
}
