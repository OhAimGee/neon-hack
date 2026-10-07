//! Rendering a [`Text`] into a string, in a language and a mode.

use super::ascii::to_ascii;
use super::catalog::Catalog;
use super::template::{Part, Template};
use super::{Arg, Text};

/// How a text is rendered: two independent choices, which can be combined (a screen-reader
/// user on a terminal that only does ASCII).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderMode {
    /// Screen-reader wording: no symbols to spell out (`key@sr`). Accents are kept.
    pub screen_reader: bool,
    /// 7-bit ASCII: the `key@ascii` variants, then transliteration of the whole line
    /// (accents, typographic symbols, and text typed by the player).
    pub ascii: bool,
}

impl RenderMode {
    /// Everything: accents, symbols, decoration.
    pub const FULL: Self = Self {
        screen_reader: false,
        ascii: false,
    };
    /// Screen-reader wording.
    pub const SCREEN_READER: Self = Self {
        screen_reader: true,
        ascii: false,
    };
    /// 7-bit ASCII.
    pub const ASCII: Self = Self {
        screen_reader: false,
        ascii: true,
    };

    /// Suffixes of the key variants tried, in this order, before the base key.
    pub fn suffixes(self) -> impl Iterator<Item = &'static str> {
        self.screen_reader
            .then_some("@sr")
            .into_iter()
            .chain(self.ascii.then_some("@ascii"))
    }
}

/// Renders a text with a catalog. A missing key shows as `<missing:key>` and an argument
/// the template asks for but the text lacks as `<?name>`, so a problem is visible, and
/// tests can assert that neither ever happens.
///
/// When the mode is ASCII the result is transliterated here, once, so that everything a
/// frontend lays out is the text that is really displayed.
#[must_use]
pub fn render(text: &Text, catalog: &Catalog, mode: RenderMode) -> String {
    let rendered = render_inner(text, catalog, mode);
    if mode.ascii {
        to_ascii(&rendered)
    } else {
        rendered
    }
}

fn render_inner(text: &Text, catalog: &Catalog, mode: RenderMode) -> String {
    let template = mode
        .suffixes()
        .find_map(|suffix| catalog.get(&format!("{}{suffix}", text.key)))
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
        render(text, &english(), RenderMode::FULL)
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
                RenderMode::FULL,
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
            render(&text, &catalog, RenderMode::SCREEN_READER),
            "Hello Case. You have 5 credits."
        );
        assert_eq!(
            render(&text, &catalog, RenderMode::ASCII),
            "Hello Case - 5 credits."
        );
        let plain = Text::new("plain");
        for mode in [RenderMode::SCREEN_READER, RenderMode::ASCII] {
            assert_eq!(render(&plain, &catalog, mode), "No placeholder.");
        }
    }

    #[test]
    fn ascii_mode_transliterates_the_whole_line_including_what_the_player_typed() {
        let catalog = english();
        let accents = Text::new("accents");
        assert_eq!(render(&accents, &catalog, RenderMode::FULL), "Café → ok");
        assert_eq!(render(&accents, &catalog, RenderMode::ASCII), "Cafe -> ok");
        let typed = Text::new("outer").with_text("item", Text::raw("Zoë « 影 »"));
        assert_eq!(
            render(&typed, &catalog, RenderMode::ASCII),
            "Bought Zoe \" ? \"."
        );
        assert_eq!(
            render(&typed, &catalog, RenderMode::SCREEN_READER),
            "Bought Zoë « 影 ».",
            "only --ascii changes the characters"
        );
    }

    #[test]
    fn screen_reader_and_ascii_combine_with_the_spoken_wording_first() {
        let catalog = catalog(
            Lang::En,
            r#"
            both = "Base é → x"
            "both@sr" = "Spoken é to x"
            "both@ascii" = "Plain é -> x"
            only_ascii = "Base é"
            "only_ascii@ascii" = "Plain é"
            only_base = "Base é →"
            "#,
        );
        let combined = RenderMode {
            screen_reader: true,
            ascii: true,
        };
        let text = |key| render(&Text::new(key), &catalog, combined);
        assert_eq!(text("both"), "Spoken e to x", "@sr wins, then it is ASCII");
        assert_eq!(text("only_ascii"), "Plain e", "no @sr: the @ascii variant");
        assert_eq!(
            text("only_base"),
            "Base e ->",
            "no variant: the base, transliterated"
        );
        let single = |mode| render(&Text::new("both"), &catalog, mode);
        assert_eq!(single(RenderMode::SCREEN_READER), "Spoken é to x");
        assert_eq!(single(RenderMode::ASCII), "Plain e -> x");
        assert_eq!(single(RenderMode::FULL), "Base é → x");
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
