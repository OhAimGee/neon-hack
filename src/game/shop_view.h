#ifndef NH_SHOP_VIEW_H
#define NH_SHOP_VIEW_H

/*
 * Vitrine de la boutique de R4Z0R : tout le catalogue d'un coup d'œil, sur deux colonnes de
 * « cases » quand l'écran est assez large, sans jamais dépasser la zone de texte.
 *
 * Pourquoi : l'ancien affichage faisait ~85 lignes (une carte de 7 lignes par objet) dans une zone
 * de 22 lignes. Le texte défilait, seuls les derniers objets restaient visibles, et le joueur
 * devait remonter dans l'historique du terminal — ce qui emmène aussi les barres du HUD.
 *
 * La composition est pure (tampon, largeur, budget de lignes) : rien n'est lu sur le terminal, donc
 * tout se teste. nh_shop_show() n'est que la colle qui interroge le terminal et affiche.
 */

#include <stddef.h>

#include "shop.h"

/* Résumé d'un objet : 33 colonnes au plus (case de 38 colonnes, moins le retrait sous « [01] »). */
#define NH_SHOP_BLURB_WIDTH 33

/* Taille de tampon suffisante pour n'importe quelle mise en page (au plus ~30 lignes de ~160 octets). */
#define NH_SHOP_BUFFER 8192

typedef enum
{
    NH_SHOP_OK,          /* achetable tout de suite */
    NH_SHOP_LOW_LEVEL,   /* niveau du joueur trop bas */
    NH_SHOP_LOW_CREDITS, /* pas assez de crédits */
    NH_SHOP_SOLD_OUT     /* objet unique déjà acheté */
} NhShopState;

/* Même ordre de vérification que buy_item() : épuisé, puis niveau, puis crédits. */
NhShopState nh_shop_state(const ShopItem *item, int credits, int level);

/* Résumé traduit d'un objet du catalogue ("" si le type est hors bornes). */
const char *nh_shop_blurb(ShopItemType type);

/*
 * Compose la vitrine dans `out` (toujours terminé par '\0', jamais coupé au milieu d'un caractère) :
 * une ligne vide, l'en-tête, les objets, une ligne vide. Retourne la taille écrite.
 *
 *  cols       largeur de l'écran (0 ou moins : 80). Aucune ligne ne dépasse cols - 1 colonnes.
 *  max_lines  lignes disponibles pour l'ensemble (0 ou moins : pas de limite). La mise en page la
 *             plus confortable qui tient est choisie : cartes aérées, cartes serrées, puis liste
 *             d'une ligne par objet. Si même la liste ne tient pas, c'est la liste qui est écrite.
 */
size_t nh_shop_compose(char *out, size_t size, const CyberShop *shop, int credits, int level,
                       int cols, int max_lines);

/*
 * Affiche la vitrine adaptée au terminal : sa largeur, et sa hauteur moins les deux barres du HUD
 * quand il est actif, moins la ligne de la question posée juste après. Sortie redirigée : 80
 * colonnes, sans limite de hauteur.
 */
void nh_shop_show(const CyberShop *shop, int credits, int level);

#endif /* NH_SHOP_VIEW_H */
