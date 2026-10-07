//! Texts of the demo game, in English and French.
//!
//! The `ui.*`, `gauge.*`, `band.*` and `contact.*` keys are the ones every frontend
//! needs; phase R1.2 moves them into the real embedded catalogs.

use crate::text::StaticCatalog;

/// The English texts of the demo.
#[must_use]
pub fn catalog_en() -> StaticCatalog {
    StaticCatalog::new(EN)
}

/// The French texts of the demo.
#[must_use]
pub fn catalog_fr() -> StaticCatalog {
    StaticCatalog::new(FR)
}

const EN: &[(&str, &str)] = &[
    ("ui.raw", "{value}"),
    ("ui.tagged", "{tag} {text}"),
    ("ui.tag.alert", "[ALERT]"),
    ("ui.tag.reward", "[Reward]"),
    ("ui.tag.error", "[Error]"),
    ("ui.say", "{speaker} » {text}"),
    ("ui.say@ascii", "{speaker}: {text}"),
    ("ui.say@sr", "{speaker}: {text}"),
    ("ui.gauge.up", "{gauge} +{delta} ({from} → {to}, {band})."),
    (
        "ui.gauge.up@ascii",
        "{gauge} +{delta} ({from} -> {to}, {band}).",
    ),
    (
        "ui.gauge.up@sr",
        "{gauge} up by {delta}, from {from} to {to}, level {band}.",
    ),
    ("ui.gauge.down", "{gauge} -{delta} ({from} → {to}, {band})."),
    (
        "ui.gauge.down@ascii",
        "{gauge} -{delta} ({from} -> {to}, {band}).",
    ),
    (
        "ui.gauge.down@sr",
        "{gauge} down by {delta}, from {from} to {to}, level {band}.",
    ),
    ("ui.prompt.command", "> "),
    ("ui.prompt.choice", "? "),
    ("ui.prompt.text", "{label}: "),
    ("ui.prompt.text_default", "{label} [{default}]: "),
    ("ui.prompt.confirm", "{question} {hint} "),
    ("ui.prompt.continue", "[Enter] to continue"),
    ("ui.hint.yes_default", "[Y/n]"),
    ("ui.hint.no_default", "[y/N]"),
    ("ui.choice.unavailable", "{label} ({reason})"),
    ("ui.sr.cell", "{column}: {value}"),
    ("ui.invalid_input", "That input is not valid here."),
    (
        "ui.invalid_choice",
        "Choose one of the listed numbers or names.",
    ),
    ("ui.invalid_confirm", "Please answer yes or no."),
    ("ui.eof", "Input closed: ending the session."),
    (
        "ui.tui.too_small",
        "Terminal too small ({width}x{height}): at least {min_width}x{min_height} needed.",
    ),
    ("ui.tui.press_key", "Press any key to leave."),
    ("gauge.trace", "Trace"),
    ("band.calm", "calm"),
    ("band.tense", "tense"),
    ("band.critical", "critical"),
    ("contact.echo7.name", "ECHO-7"),
    ("demo.banner.alt", "NEON HACK"),
    (
        "demo.intro.1",
        "Neo-Tokyo, 2087. An old cyberdeck boots up on its own.",
    ),
    (
        "demo.intro.2",
        "On the screen, a single line: ENCRYPTED CHANNEL #7 - INCOMING CONNECTION.",
    ),
    ("demo.prompt.name", "Your handle"),
    ("demo.prompt.confirm_name", "Carve \"{name}\" into the net?"),
    (
        "demo.echo.greeting",
        "Finally. I was starting to think you would never wake up, {name}.",
    ),
    ("demo.echo.ready", "Type 'help' to see what you can do."),
    ("demo.help.title", "Commands"),
    ("demo.help.col_command", "Command"),
    ("demo.help.col_effect", "Effect"),
    ("demo.help.help", "List the commands"),
    ("demo.help.status", "Show your situation"),
    ("demo.help.scan", "Scan the network"),
    ("demo.help.shop", "Visit the stall"),
    ("demo.help.quit", "Leave the net"),
    ("demo.status.name", "Handle: {name}"),
    ("demo.status.credits", "Credits: {credits}"),
    ("demo.status.trace", "Trace: {value}/{max} ({band})"),
    ("demo.scan.found", "The scan finds {found} open ports."),
    ("demo.scan.reward", "+{credits} credits."),
    ("demo.alert.tense", "Security has noticed you."),
    (
        "demo.alert.critical",
        "Security is closing in: your trace is critical.",
    ),
    ("demo.shop.title", "R4Z0R's stall"),
    ("demo.shop.cancel", "Leave the stall"),
    ("demo.item.proxy", "Proxy chain ({price} credits)"),
    ("demo.item.cloak", "Cloak module ({price} credits)"),
    ("demo.item.deck", "Deck upgrade ({price} credits)"),
    ("demo.item.proxy.name", "the proxy chain"),
    ("demo.item.cloak.name", "the cloak module"),
    ("demo.item.deck.name", "the deck upgrade"),
    ("demo.shop.owned", "You already own it."),
    ("demo.shop.cannot_afford", "{missing} credits short."),
    ("demo.shop.bought", "You bought {item}."),
    (
        "demo.shop.proxy_effect",
        "The proxy chain scrubs your trail.",
    ),
    ("demo.quit.confirm", "Leave the net?"),
    ("demo.quit.bye", "Goodbye, {name}."),
    ("demo.unknown_command", "Unknown command: {command}"),
    ("demo.objective.trace", "Keep your trace under {limit}."),
];

const FR: &[(&str, &str)] = &[
    ("ui.raw", "{value}"),
    ("ui.tagged", "{tag} {text}"),
    ("ui.tag.alert", "[ALERTE]"),
    ("ui.tag.reward", "[Gain]"),
    ("ui.tag.error", "[Erreur]"),
    ("ui.say", "{speaker} » {text}"),
    ("ui.say@ascii", "{speaker}: {text}"),
    ("ui.say@sr", "{speaker}: {text}"),
    ("ui.gauge.up", "{gauge} +{delta} ({from} → {to}, {band})."),
    (
        "ui.gauge.up@ascii",
        "{gauge} +{delta} ({from} -> {to}, {band}).",
    ),
    (
        "ui.gauge.up@sr",
        "{gauge} en hausse de {delta}, de {from} à {to}, niveau {band}.",
    ),
    ("ui.gauge.down", "{gauge} -{delta} ({from} → {to}, {band})."),
    (
        "ui.gauge.down@ascii",
        "{gauge} -{delta} ({from} -> {to}, {band}).",
    ),
    (
        "ui.gauge.down@sr",
        "{gauge} en baisse de {delta}, de {from} à {to}, niveau {band}.",
    ),
    ("ui.prompt.command", "> "),
    ("ui.prompt.choice", "? "),
    ("ui.prompt.text", "{label} : "),
    ("ui.prompt.text_default", "{label} [{default}] : "),
    ("ui.prompt.confirm", "{question} {hint} "),
    ("ui.prompt.continue", "[Entrée] pour continuer"),
    ("ui.hint.yes_default", "[O/n]"),
    ("ui.hint.no_default", "[o/N]"),
    ("ui.choice.unavailable", "{label} ({reason})"),
    ("ui.sr.cell", "{column} : {value}"),
    ("ui.invalid_input", "Cette saisie n'est pas valable ici."),
    (
        "ui.invalid_choice",
        "Choisissez un des numéros ou des noms listés.",
    ),
    ("ui.invalid_confirm", "Répondez par oui ou par non."),
    ("ui.eof", "Entrée fermée : fin de la session."),
    (
        "ui.tui.too_small",
        "Terminal trop petit ({width}x{height}) : {min_width}x{min_height} au minimum.",
    ),
    ("ui.tui.press_key", "Appuyez sur une touche pour quitter."),
    ("gauge.trace", "Trace"),
    ("band.calm", "calme"),
    ("band.tense", "tendu"),
    ("band.critical", "critique"),
    ("contact.echo7.name", "ECHO-7"),
    ("demo.banner.alt", "NEON HACK"),
    (
        "demo.intro.1",
        "Neo-Tokyo, 2087. Un vieux cyberdeck s'allume tout seul.",
    ),
    (
        "demo.intro.2",
        "À l'écran, une seule ligne : CANAL CHIFFRÉ #7 - CONNEXION ENTRANTE.",
    ),
    ("demo.prompt.name", "Votre pseudo"),
    (
        "demo.prompt.confirm_name",
        "Graver « {name} » dans le réseau ?",
    ),
    (
        "demo.echo.greeting",
        "Enfin. Je commençais à croire que tu ne te réveillerais jamais, {name}.",
    ),
    (
        "demo.echo.ready",
        "Tape 'help' pour voir ce que tu peux faire.",
    ),
    ("demo.help.title", "Commandes"),
    ("demo.help.col_command", "Commande"),
    ("demo.help.col_effect", "Effet"),
    ("demo.help.help", "Lister les commandes"),
    ("demo.help.status", "Voir votre situation"),
    ("demo.help.scan", "Scanner le réseau"),
    ("demo.help.shop", "Visiter l'étal"),
    ("demo.help.quit", "Quitter le réseau"),
    ("demo.status.name", "Pseudo : {name}"),
    ("demo.status.credits", "Crédits : {credits}"),
    ("demo.status.trace", "Trace : {value}/{max} ({band})"),
    ("demo.scan.found", "Le scan trouve {found} ports ouverts."),
    ("demo.scan.reward", "+{credits} crédits."),
    ("demo.alert.tense", "La sécurité vous a repéré."),
    (
        "demo.alert.critical",
        "La sécurité se rapproche : votre trace est critique.",
    ),
    ("demo.shop.title", "L'étal de R4Z0R"),
    ("demo.shop.cancel", "Quitter l'étal"),
    ("demo.item.proxy", "Chaîne de proxys ({price} crédits)"),
    ("demo.item.cloak", "Module de camouflage ({price} crédits)"),
    ("demo.item.deck", "Amélioration du deck ({price} crédits)"),
    ("demo.item.proxy.name", "la chaîne de proxys"),
    ("demo.item.cloak.name", "le module de camouflage"),
    ("demo.item.deck.name", "l'amélioration du deck"),
    ("demo.shop.owned", "Vous le possédez déjà."),
    ("demo.shop.cannot_afford", "Il manque {missing} crédits."),
    ("demo.shop.bought", "Vous avez acheté {item}."),
    (
        "demo.shop.proxy_effect",
        "La chaîne de proxys efface vos traces.",
    ),
    ("demo.quit.confirm", "Quitter le réseau ?"),
    ("demo.quit.bye", "Au revoir, {name}."),
    ("demo.unknown_command", "Commande inconnue : {command}"),
    ("demo.objective.trace", "Gardez votre trace sous {limit}."),
];
