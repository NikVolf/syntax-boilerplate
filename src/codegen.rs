use aster;

use syntax::ast::{
	Ident,
	MetaItem,
	Item,
	ImplItemKind,
	ImplItem,
	MethodSig,
	Arg,
	Pat,
	PatKind,
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

    let impl_item = match implement_interface(cx, &builder, &item, push) {
        Ok(item) => item,
        Err(Error) => {
            // An error occured, but it should have been reported already.
            return;
        }
    };

    push(Annotatable::Item(impl_item))
}

fn implement_param(
	cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
	implement: &ImplItem,
	signature: &MethodSig,
	arg: &Arg,
    push: &mut FnMut(Annotatable),
) {

}

fn push_invoke_signature (
	cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
	implement: &ImplItem,
	signature: &MethodSig,
    push: &mut FnMut(Annotatable),
) {
	let name_str = format!("{}_input", implement.ident.name.as_str());
	let name = builder.id(builder.name(name_str.as_str()));

	let field_name_str = format!("{}_input", implement.ident.name.as_str());
	let field_name = builder.id(builder.name(field_name_str.as_str()));

	let ty = quote_ty!(cx, usize);

	let input_struct =
		quote_item!(cx,
			struct $name {
				$field_name: $ty
			}
	    ).unwrap();

	push(Annotatable::Item(input_struct));
}

fn field_name(builder: &aster::AstBuilder, arg: &Arg) -> ast::Ident {
	match arg.pat.node {
		 PatKind::Ident(_, ref ident, _) => builder.id(ident.node),
		_ => { panic!("unexpected param in interface: {:?}", arg.pat.node) }
	}
}

fn push_invoke_signature_aster (
	cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
	implement: &ImplItem,
	signature: &MethodSig,
    push: &mut FnMut(Annotatable),
) {
	let name_str = format!("{}_input", implement.ident.name.as_str());

	let inputs = &signature.decl.inputs;
	if inputs.len() > 0 {
		let arg = &inputs[0];
		let mut tree = builder.item().struct_(name_str.as_str())
			.field(format!("field_{}", field_name(builder, &arg).name.as_str())).ty().build(arg.ty.clone());

		for arg in inputs.iter().skip(1) {
			tree = tree.field(format!("field_{}", field_name(builder, &arg).name.as_str())).ty().build(arg.ty.clone());
		}
		push(Annotatable::Item(tree.build()));
	}
	else {
		push(Annotatable::Item(builder.item().struct_(name_str.as_str()).build()));
	}
}

//
//
//fn push_invoke_signature_ast (
//	cx: &ExtCtxt,
//    builder: &aster::AstBuilder,
//    item: &Item,
//	implement: &ImplItem,
//	signature: &MethodSig,
//    push: &mut FnMut(Annotatable),
//) {
//	use syntax::ast::*;
//	use syntax;
//
//	let field_name_str = format!("{}_field1", implement.ident.name.as_str());
//	let field_name = builder.id(builder.name(field_name_str.as_str()));
//
//	let struct_field_ty = builder.ty().id("usize");
//
//	let struct_field = syntax::codemap::Spanned {
//		node: StructField {
//			kind: StructFieldKind::Named(field_name, Visibility::Public),
//			id: DUMMY_NODE_ID,
//			ty: struct_field_ty,
//			attrs: vec![],
//		},
//		span: syntax::codemap::DUMMY_SP,
//	};
//
//	let struct_def = syntax::ptr::P(syntax::ast::StructDef {
//		fields: vec![struct_field],
//		ctor_id: None,
//	});
//
//	let struct_item = Item {
//		ident: field_name,
//		attrs: vec![],
//		id: DUMMY_NODE_ID,
//		node: syntax::ast::Item_::ItemStruct(P::from_vec(vec![struct_def])),
//		span: syntax::codemap::DUMMY_SP
//	};
//
//	push(Annotatable::Item(syntax::ptr::P(struct_item)));
//}

fn implement_interface(
    cx: &ExtCtxt,
    builder: &aster::AstBuilder,
    item: &Item,
    push: &mut FnMut(Annotatable),
) -> Result<P<ast::Item>, Error> {
    let (generics, impl_items) = match item.node {
        ast::ItemKind::Impl(_, _, ref generics, _, _, ref impl_items) => (generics, impl_items),
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

	for impl_item in impl_items {
		if let ImplItemKind::Method(ref signature, ref block) = impl_item.node {
			push_invoke_signature_aster(cx, builder, item, &impl_item, signature, push);
		}
	}

    Ok(quote_item!(cx,
        impl $impl_generics ::codegen::interface::IpcInterface<$ty> for $ty $where_clause {
			fn dispatch(&self)
            {
            }
        }
    ).unwrap())
}
