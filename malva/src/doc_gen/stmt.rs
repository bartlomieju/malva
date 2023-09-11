use super::DocGen;
use crate::ctx::Ctx;
use raffia::{
    ast::*,
    token::{CommentKind, TokenWithSpan},
    Spanned, Syntax,
};
use tiny_pretty::Doc;

impl<'s> DocGen<'s> for Declaration<'s> {
    fn doc(&self, ctx: &Ctx<'_, 's>) -> Doc<'s> {
        let mut docs = Vec::with_capacity(3);
        docs.push(self.name.doc(ctx));
        if let Some(less_property_merge) = &self.less_property_merge {
            docs.push(less_property_merge.doc(ctx));
        }
        docs.push(Doc::text(": "));

        let mut values = Vec::with_capacity(self.value.len() * 2);

        let mut iter = self.value.iter().peekable();
        match &self.name {
            InterpolableIdent::Literal(Ident { name, .. })
                if name.starts_with("--") || name.eq_ignore_ascii_case("filter") =>
            {
                use raffia::token::Token;
                let mut end = self.colon_span.end;
                while let Some(value) = iter.next() {
                    let span = value.span();
                    let comments = ctx.get_comments_between(end, span.start);
                    comments.for_each(|comment| {
                        values.push(comment.doc(ctx));
                        if matches!(comment.kind, CommentKind::Block) {
                            values.push(Doc::soft_line());
                        }
                    });

                    values.push(value.doc(ctx));
                    if let ComponentValue::TokenWithSpan(TokenWithSpan {
                        token: Token::Comma(..) | Token::Semicolon(..),
                        ..
                    }) = value
                    {
                        values.push(Doc::soft_line());
                    } else if matches!(iter.peek(), Some(next) if value.span().end < next.span().start)
                    {
                        values.push(Doc::soft_line());
                    }

                    end = span.end;
                }
            }
            _ => {
                let mut end = self.colon_span.end;
                while let Some(value) = iter.next() {
                    let span = value.span();
                    let comments = ctx.get_comments_between(end, span.start);
                    comments.for_each(|comment| {
                        values.push(comment.doc(ctx));
                        if matches!(comment.kind, CommentKind::Block) {
                            values.push(Doc::soft_line());
                        }
                    });

                    values.push(value.doc(ctx));
                    if !matches!(
                        iter.peek(),
                        Some(ComponentValue::Delimiter(Delimiter {
                            kind: DelimiterKind::Comma | DelimiterKind::Semicolon,
                            ..
                        })) | None
                    ) {
                        values.push(Doc::soft_line());
                    }

                    end = span.end;
                }
            }
        }

        if let Some(important) = &self.important {
            values.push(Doc::soft_line());
            values.push(important.doc(ctx));
        }

        docs.push(Doc::list(values).group().nest(ctx.indent_width));

        Doc::list(docs)
    }
}

impl<'s> DocGen<'s> for ImportantAnnotation<'s> {
    fn doc(&self, _: &Ctx<'_, 's>) -> Doc<'s> {
        Doc::text("!important")
    }
}

impl<'s> DocGen<'s> for QualifiedRule<'s> {
    fn doc(&self, ctx: &Ctx<'_, 's>) -> Doc<'s> {
        use crate::config::BlockSelectorLineBreak;

        // we don't use `SelectorList::doc` here
        // because it's a special case for qualified rule
        Doc::list(
            itertools::intersperse(
                self.selector
                    .selectors
                    .iter()
                    .map(|selector| selector.doc(ctx)),
                Doc::text(",").append(match ctx.options.block_selector_linebreak {
                    BlockSelectorLineBreak::Always => Doc::hard_line(),
                    BlockSelectorLineBreak::Consistent => Doc::line_or_space(),
                    BlockSelectorLineBreak::Wrap => Doc::soft_line(),
                }),
            )
            .collect(),
        )
        .group()
        .append(Doc::space())
        .append(self.block.doc(ctx))
    }
}

impl<'s> DocGen<'s> for SimpleBlock<'s> {
    fn doc(&self, ctx: &Ctx<'_, 's>) -> Doc<'s> {
        let is_sass = ctx.syntax == Syntax::Sass;
        let mut docs = vec![];

        if !is_sass {
            docs.push(Doc::text("{"));
        }

        let mut stmts = Vec::with_capacity(self.statements.len() * 2);
        let mut iter = self.statements.iter().peekable();
        while let Some(stmt) = iter.next() {
            stmts.push(Doc::hard_line());
            stmts.push(stmt.doc(ctx));
            if let Some(next) = iter.peek() {
                if ctx
                    .line_bounds
                    .is_away_more_than_one_line(stmt.span().end - 1, next.span().start)
                {
                    stmts.push(Doc::empty_line());
                }
            }
        }
        docs.push(Doc::list(stmts).nest(ctx.indent_width));
        docs.push(Doc::hard_line());

        if !is_sass {
            docs.push(Doc::text("}"));
        }

        Doc::list(docs)
    }
}

impl<'s> DocGen<'s> for Statement<'s> {
    fn doc(&self, ctx: &Ctx<'_, 's>) -> Doc<'s> {
        let stmt = match self {
            Statement::AtRule(at_rule) => at_rule.doc(ctx),
            Statement::Declaration(declaration) => declaration.doc(ctx),
            Statement::KeyframeBlock(keyframe_block) => keyframe_block.doc(ctx),
            Statement::QualifiedRule(qualified_rule) => qualified_rule.doc(ctx),
            _ => todo!(),
        };
        if ctx.syntax == Syntax::Sass {
            stmt
        } else {
            match self {
                Statement::AtRule(at_rule) if at_rule.block.is_none() => {
                    stmt.append(Doc::text(";"))
                }
                Statement::Declaration(decl)
                    if !matches!(
                        decl.value.last(),
                        Some(ComponentValue::SassNestingDeclaration(..))
                    ) =>
                {
                    stmt.append(Doc::text(";"))
                }
                _ => stmt,
            }
        }
    }
}

impl<'s> DocGen<'s> for Stylesheet<'s> {
    fn doc(&self, ctx: &Ctx<'_, 's>) -> Doc<'s> {
        let mut stmts = Vec::with_capacity(self.statements.len() * 2);
        let mut iter = self.statements.iter().peekable();
        while let Some(stmt) = iter.next() {
            stmts.push(stmt.doc(ctx));
            if let Some(next) = iter.peek() {
                stmts.push(Doc::hard_line());
                if ctx
                    .line_bounds
                    .is_away_more_than_one_line(stmt.span().end - 1, next.span().start)
                {
                    stmts.push(Doc::empty_line());
                }
            } else if ctx.syntax != Syntax::Sass {
                stmts.push(Doc::hard_line());
            }
        }
        Doc::list(stmts)
    }
}
