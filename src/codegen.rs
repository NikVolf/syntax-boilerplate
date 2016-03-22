use aster;

use syntax::ast::{
    Ident,
    MetaItem,
    Item,
};

use syntax::ast;
use syntax::codemap::Span;
use syntax::ext::base::{Annotatable, ExtCtxt};
use syntax::ext::build::AstBuilder;
use syntax::ptr::P;

pub struct Error;

pub fn expand_ipc_implementation(
    cx: &mut ExtCtxt,
    span: Span,
    meta_item: &MetaItem,
    annotatable: &Annotatable,
    push: &mut FnMut(Annotatable)
) {
    let item = match *annotatable {
        Annotatable::Item(ref item) => item,
        _ => {
            cx.span_err(
                meta_item.span,
                "`#[derive(Ipc)]` may only be applied to struct implementations");
            return;
        }
    };

    let builder = aster::AstBuilder::new().span(span);

    let impl_item = match implement_function(cx, &builder, &item) {
        Ok(item) => item,
        Err(Error) => {
            // An error occured, but it should have been reported already.
            return;
        }
    };

    push(Annotatable::Item(impl_item))
}

fn implement_function(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
) -> Result<P<ast::Item>, Error> {
    let generics = match item.node {
        ast::ItemKind::Impl(_, _, ref generics, _, _, _) => generics,
        _ => {
            cx.span_err(
                item.span,
                "`#[derive(Ipc)]` may only be applied to struct implementations");
            return Err(Error);
        }
    };

    let impl_generics = builder.from_generics(generics.clone())
        .add_ty_param_bound(
            builder.path().global().ids(&["ethcore_ipc"]).build()
        )
        .build();

    let ty = builder.ty().path()
        .segment(item.ident).with_generics(impl_generics.clone()).build()
        .build();

    let where_clause = &impl_generics.where_clause;

    Ok(quote_item!(cx,
        impl $impl_generics ::codegen::interface::IpcInterface<$ty> for $ty $where_clause {
            fn call(&self)
            {
            }
        }
    ).unwrap())
}
