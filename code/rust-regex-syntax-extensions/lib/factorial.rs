#![crate_type = "dylib"]
#![feature(plugin_registrar, managed_boxes, quote)]

extern crate rustc;
extern crate syntax;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{ExtCtxt, MacResult, MacExpr};
use rustc::plugin::Registry;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("factorial", expand)
}

fn expand(cx: &mut ExtCtxt, _: codemap::Span, _: &[ast::TokenTree]) -> Box<MacResult> {
    let answer = factorial(5 as u64);
    MacExpr::new(quote_expr!(cx, $answer))
}

fn factorial(n: u64) -> u64 {
    // Brings the 'product' method from the MultiplicativeIterator trait
    // into scope.
    use std::iter::MultiplicativeIterator;
    range(2, n+1).product()
}
