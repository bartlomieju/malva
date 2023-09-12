pub mod config;
mod ctx;
mod doc_gen;
mod error;
mod line_bounds;

use crate::{config::FormatOptions, ctx::Ctx};
use doc_gen::DocGen;
pub use error::Error;
pub use line_bounds::LineBounds;
pub use raffia::Syntax;
use raffia::{ast::Stylesheet, token::Comment, ParserBuilder};

pub fn format_text(input: &str, syntax: Syntax, options: &FormatOptions) -> Result<String, Error> {
    let line_bounds = LineBounds::new(input);
    let mut comments = vec![];
    let mut parser = ParserBuilder::new(&input)
        .syntax(syntax)
        .comments(&mut comments)
        .build();
    let stylesheet = match parser.parse::<Stylesheet>() {
        Ok(stylesheet) => stylesheet,
        Err(error) => return Err(error.into()),
    };

    Ok(print_stylesheet(
        &stylesheet,
        &comments,
        line_bounds,
        syntax,
        options,
    ))
}

pub fn print_stylesheet<'a, 's>(
    stylesheet: &'a Stylesheet<'s>,
    comments: &'a [Comment<'s>],
    line_bounds: LineBounds,
    syntax: Syntax,
    options: &'a FormatOptions,
) -> String {
    use tiny_pretty::{IndentKind, PrintOptions};

    let ctx = Ctx {
        syntax,
        options: &options.language,
        comments: &comments,
        indent_width: options.layout.indent_width,
        line_bounds,
    };
    let doc = stylesheet.doc(&ctx);
    tiny_pretty::print(
        &doc,
        &PrintOptions {
            indent_kind: if options.layout.use_tabs {
                IndentKind::Tab
            } else {
                IndentKind::Space
            },
            line_break: options.layout.line_break.clone().into(),
            width: options.layout.print_width,
            tab_size: options.layout.indent_width,
        },
    )
}
