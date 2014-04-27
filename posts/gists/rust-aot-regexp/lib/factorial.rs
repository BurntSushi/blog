#![crate_type = "dylib"]
#![feature(macro_registrar, managed_boxes, quote)]

extern crate syntax;

use syntax::ast;
use syntax::codemap;
use syntax::ext::base::{
    SyntaxExtension, ExtCtxt, MacResult, MacExpr,
    NormalTT, BasicMacroExpander,
};
use syntax::parse::token;

#[macro_registrar]
pub fn macro_registrar(register: |ast::Name, SyntaxExtension|) {
    let expander = ~BasicMacroExpander { expander: expand, span: None };
    register(token::intern("factorial"), NormalTT(expander, None))
}

fn expand(cx: &mut ExtCtxt, _: codemap::Span, _: &[ast::TokenTree]) -> ~MacResult {
    let answer = factorial(5 as u64);
    MacExpr::new(quote_expr!(cx, $answer))
}

fn factorial(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => n * factorial(n - 1),
    }
}
