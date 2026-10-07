//! Rendering a [`Text`] into a string, in a language and a mode.

use super::ascii::to_ascii;
use super::catalog::Catalog;
use super::template::{Part, Template};
use super::{Arg, Text};

/// How a text is rendered. The mode selects a variant of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderMode {
    /// Everything: accents, symbols, decoration.
    #[default]
    Full,
    /// Screen-reader wording: no symbols to spell out (`key@sr`).
    ScreenReader,
    /// 7-bit ASCII: the `key@ascii` variants, then transliteration of the whole line
    /// (accents, typographic symbols, and text typed by the player).
    Ascii,
}

impl RenderMode {
    /// Suffix of the key variant tried before the base key.
    #[must_use]
    pub fn suffix(self) -> Option<&'static str> {
        match self {
            Self::Full => None,
            Self::ScreenReader => Some("@sr"),
            Self::Ascii => Some("@ascii"),
        }
    }
}

/// Renders a text with a catalog. A missing key shows as `<missing:key>` and an argument
/// the template asks for but the text lacks as `<?name>`, so a problem is visible, and
/// tests can assert that neither ever happens.
///
/// In [`RenderMode::Ascii`] the result is transliterated here, once, so that everything a
/// frontend lays out is the text that is really displayed.
#[must_use]
pub fn render(text: &Text, catalog: &Catalog, mode: RenderMode) -> String {
    let rendered = render_inner(text, catalog, mode);
    if mode == RenderMode::Ascii {
        to_ascii(&rendered)
    } else {
        rendered
    }
}

fn render_inner(text: &Text, catalog: &Catalog, mode: RenderMode) -> String {
    let template = mode
        .suffix()
        .and_then(|suffix| catalog.get(&format!("{}{suffix}", text.key)))
        .or_else(|| catalog.get(&text.key));
    let Some(template) = template else {
        return format!("<missing:{}>", text.key);
    };
    let mut out = String::new();
    push_template(template, &text.args, catalog, mode, &mut out);
    out
}

fn push_template(
    template: &Template,
    args: &[(&'static str, Arg)],
    catalog: &Catalog,
    mode: RenderMode,
    out: &mut String,
) {
    let find = |name: &str| {
        args.iter()
            .find(|(arg_name, _)| *arg_name == name)
            .map(|(_, arg)| arg)
    };
    for part in &template.parts {
        match part {
            Part::Literal(literal) => out.push_str(literal),
            Part::Placeholder(name) => match find(name) {
                Some(arg) => push_arg(arg, catalog, mode, out),
                None => push_unknown(name, out),
            },
            Part::Plural {
                selector,
                one,
                other,
            } => match find(selector) {
                Some(Arg::Int(count)) => {
                    let form = match catalog.lang().plural(*count) {
                        super::catalog::PluralForm::One => one,
                        super::catalog::PluralForm::Other => other,
                    };
                    push_template(form, args, catalog, mode, out);
                }
                // A plural of something that is not a number cannot pick a form.
                _ => push_unknown(selector, out),
            },
        }
    }
}

fn push_unknown(name: &str, out: &mut String) {
    out.push_str("<?");
    out.push_str(name);
    out.push('>');
}

fn push_arg(arg: &Arg, catalog: &Catalog, mode: RenderMode, out: &mut String) {
    match arg {
        Arg::Int(value) => out.push_str(&value.to_string()),
        Arg::Str(value) => out.push_str(value),
        Arg::Term(term) => {
            let key = format!("term.{term}");
            out.push_str(&render_inner(&Text::dynamic(key), catalog, mode));
        }
        Arg::Text(text) => out.push_str(&render_inner(text, catalog, mode)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Lang;

    fn catalog(lang: Lang, source: &str) -> Catalog {
        Catalog::from_sources(lang, &[("test.toml", source)]).unwrap()
    }

    fn english() -> Catalog {
        catalog(
            Lang::En,
            r#"
            hello = "Hello {name}, you have {n} {n|credit|credits}."
            "hello@sr" = "Hello {name}. You have {n} {n|credit|credits}."
            "hello@ascii" = "Hello {name} - {n} credits."
            plain = "No placeholder."
            outer = "Bought {item}."
            with_term = "Watch the {gauge}."
            lone = "{n|one node|{n} nodes}"
            unknown_arg = "Value: {nothing}."
            accents = "Café → ok"
            "ui.raw" = "{value}"
            [item]
            proxy = "a proxy"
            [term]
            trace = "Trace"
            "#,
        )
    }

    fn french() -> Catalog {
        catalog(Lang::Fr, "credits = \"{n} {n|crédit|crédits}\"\n")
    }

    fn full(text: &Text) -> String {
        render(text, &english(), RenderMode::Full)
    }

    #[test]
    fn substitutes_typed_arguments() {
        let text = Text::new("hello")
            .with_str("name", "Case")
            .with_int("n", 50);
        assert_eq!(full(&text), "Hello Case, you have 50 credits.");
    }

    #[test]
    fn nested_texts_and_terms_are_rendered_too() {
        let nested = Text::new("outer").with_text("item", Text::new("item.proxy"));
        assert_eq!(full(&nested), "Bought a proxy.");
        let term = Text::new("with_term").with_term("gauge", "trace");
        assert_eq!(full(&term), "Watch the Trace.");
    }

    #[test]
    fn raw_text_is_shown_unchanged() {
        assert_eq!(full(&Text::raw("ECHO-7")), "ECHO-7");
    }

    #[test]
    fn plurals_pick_their_form_with_the_rule_of_the_language() {
        let credits = |n| Text::new("hello").with_str("name", "A").with_int("n", n);
        assert_eq!(full(&credits(1)), "Hello A, you have 1 credit.");
        assert_eq!(full(&credits(0)), "Hello A, you have 0 credits.");
        assert_eq!(full(&credits(2)), "Hello A, you have 2 credits.");
        let lone = |n| full(&Text::new("lone").with_int("n", n));
        assert_eq!((lone(1), lone(7)), ("one node".into(), "7 nodes".into()));

        let french = french();
        let fr = |n| {
            render(
                &Text::new("credits").with_int("n", n),
                &french,
                RenderMode::Full,
            )
        };
        assert_eq!(fr(0), "0 crédit", "zero is singular in French");
        assert_eq!(fr(1), "1 crédit");
        assert_eq!(fr(2), "2 crédits");
    }

    #[test]
    fn mode_picks_its_variant_and_falls_back_to_the_base_key() {
        let catalog = english();
        let text = Text::new("hello").with_str("name", "Case").with_int("n", 5);
        assert_eq!(
            render(&text, &catalog, RenderMode::ScreenReader),
            "Hello Case. You have 5 credits."
        );
        assert_eq!(
            render(&text, &catalog, RenderMode::Ascii),
            "Hello Case - 5 credits."
        );
        let plain = Text::new("plain");
        for mode in [RenderMode::ScreenReader, RenderMode::Ascii] {
            assert_eq!(render(&plain, &catalog, mode), "No placeholder.");
        }
    }

    #[test]
    fn ascii_mode_transliterates_the_whole_line_including_what_the_player_typed() {
        let catalog = english();
        let accents = Text::new("accents");
        assert_eq!(render(&accents, &catalog, RenderMode::Full), "Café → ok");
        assert_eq!(render(&accents, &catalog, RenderMode::Ascii), "Cafe -> ok");
        let typed = Text::new("outer").with_text("item", Text::raw("Zoë « 影 »"));
        assert_eq!(
            render(&typed, &catalog, RenderMode::Ascii),
            "Bought Zoe \" ? \"."
        );
        assert_eq!(
            render(&typed, &catalog, RenderMode::ScreenReader),
            "Bought Zoë « 影 ».",
            "only --ascii changes the characters"
        );
    }

    #[test]
    fn problems_are_visible_not_silent() {
        assert_eq!(full(&Text::new("nope")), "<missing:nope>");
        assert_eq!(full(&Text::new("unknown_arg")), "Value: <?nothing>.");
        // A plural needs a number, and an argument that is absent or not a number is reported.
        assert_eq!(full(&Text::new("lone")), "<?n>");
        assert_eq!(full(&Text::new("lone").with_str("n", "7")), "<?n>");
    }
}
