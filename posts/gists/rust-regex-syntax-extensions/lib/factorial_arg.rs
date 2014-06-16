#![crate_type = "dylib"]
#![feature(plugin_registrar, quote)]

extern crate rustc;
extern crate syntax;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr, DummyResult};
use syntax::parse;
use syntax::parse::token;
use rustc::plugin::Registry;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("factorial", expand)
}

fn expand(cx: &mut ExtCtxt, sp: codemap::Span, tts: &[ast::TokenTree]) -> Box<MacResult> {
    let n = match parse(cx, tts) {
        Some(n) => n,
        None => return DummyResult::expr(sp),
    };
    let answer = factorial(n);
    MacExpr::new(quote_expr!(cx, $answer))
}

fn factorial(n: u64) -> u64 {
    use std::iter::MultiplicativeIterator;
    range(2, n+1).product()
}

fn parse(cx: &mut ExtCtxt, tts: &[ast::TokenTree]) -> Option<u64> {
    use syntax::print::pprust;

    let mut parser = parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(),
                                                Vec::from_slice(tts));
    // The `expand_expr` method is called so that any macro calls in the
    // parsed expression are expanded. This for example allows us to write
    // `factorial!(some_other_macro!(10u))`.
    let arg = cx.expand_expr(parser.parse_expr());
    match arg.node {
        ast::ExprLit(spanned) => {
            match spanned.node {
                ast::LitUint(n, _) => {
                    if !parser.eat(&token::EOF) {
                        cx.span_err(parser.span,
                                    "expected only one integer literal");
                        return None
                    }
                    return Some(n)
                }
                _ => {}
            }
        }
        _ => {}
    }

    let err = format!("expected unsigned integer literal but got `{}`",
                      pprust::expr_to_str(arg));
    cx.span_err(parser.span, err.as_slice());
    None
}
