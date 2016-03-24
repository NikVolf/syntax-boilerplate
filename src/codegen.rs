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
	FunctionRetTy,
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
            cx.span_err(meta_item.span, "`#[derive(Ipc)]` may only be applied to struct implementations");
            return;
        }
    };

    let builder = aster::AstBuilder::new().span(span);

    let impl_item = match implement_interface(cx, &builder, &item, push) {
        Ok(item) => item,
        Err(Error) => { return; }
    };

    push(Annotatable::Item(impl_item))
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
) -> Dispatch {

	let inputs = &signature.decl.inputs;
	let (input_type_name, input_arg_names) = if inputs.len() > 0 {
		let first_field_name = field_name(builder, &inputs[0]).name.as_str();
		if first_field_name == "self" && inputs.len() == 1 { (None, vec![]) }
		else {
			let skip = if first_field_name == "self" { 2 } else { 1 };
			let name_str = format!("{}_input", implement.ident.name.as_str());

			let mut arg_names = Vec::new();
			let arg_name = format!("{}", field_name(builder, &inputs[skip-1]).name);
			let mut tree = builder.item().attr().word("derive(Serialize, Deserialize)").struct_(name_str.as_str())
				.field(arg_name.as_str()).ty().build(inputs[skip-1].ty.clone());
			arg_names.push(arg_name);
			for arg in inputs.iter().skip(skip) {
				let arg_name = format!("{}", field_name(builder, &arg));
				tree = tree.field(arg_name.as_str()).ty().build(arg.ty.clone());
				arg_names.push(arg_name);
			}

			push(Annotatable::Item(tree.build()));
			(Some(name_str.to_owned()), arg_names)
		}
	}
	else {
		(None, vec![])
	};

	let return_type_name = match signature.decl.output {
		FunctionRetTy::Ty(ref ty) => {
			let name_str = format!("{}_output", implement.ident.name.as_str());
			let tree = builder.item().attr().word("derive(Serialize, Deserialize)").struct_(name_str.as_str())
				.field(format!("payload")).ty().build(ty.clone());
			push(Annotatable::Item(tree.build()));
			Some(name_str.to_owned())
		}
		_ => None
	};

	Dispatch {
		function_name: format!("{}", implement.ident.name.as_str()),
		input_type_name: input_type_name,
		input_arg_names: input_arg_names,
		return_type_name: return_type_name,
	}
}

struct Dispatch {
	function_name: String,
	input_type_name: Option<String>,
	input_arg_names: Vec<String>,
	return_type_name: Option<String>,
}

fn implement_dispatch_arm(
	cx: &ExtCtxt,
    builder: &aster::AstBuilder,
	dispatch: &Dispatch,
) -> P<ast::Expr>
{
	let deserialize_expr = quote_expr!(cx, ::bincode::serde::deserialize_from(r, ::bincode::SizeLimit::Infinite).expect("ipc deserialization error, aborting"));
	let input_type_id = builder.id(dispatch.input_type_name.clone().unwrap().as_str());
	let output_type_id = builder.id(dispatch.return_type_name.clone().unwrap().as_str());
	let function_name = builder.id(dispatch.function_name.as_str());

	let input_args_exprs = dispatch.input_arg_names.iter().map(|ref arg_name| {
		let arg_ident = builder.id(arg_name);
		quote_expr!(cx, input. $arg_ident)
	}).collect::<Vec<P<ast::Expr>>>();


	//	This is the expanded version of this:
	//
	//	let invoke_serialize_stmt = quote_stmt!(cx, {
	//		::bincode::serde::serialize(& $output_type_id { payload: self. $function_name ($hand_param_a, $hand_param_b) }, ::bincode::SizeLimit::Infinite).unwrap()
	//	});
	//
	//	But the above does not allow comma-separated expressions
	let invoke_serialize_stmt = {
		let ext_cx = &*cx;
		::quasi::parse_stmt_panic(&mut ::syntax::parse::new_parser_from_tts(
			ext_cx.parse_sess(),
			ext_cx.cfg(),
			{
				let _sp = ext_cx.call_site();
				let mut tt = ::std::vec::Vec::new();
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::OpenDelim(::syntax::parse::token::Brace)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::ModSep));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("bincode"), ::syntax::parse::token::ModName)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::ModSep));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("serde"), ::syntax::parse::token::ModName)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::ModSep));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("serialize"), ::syntax::parse::token::Plain)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::OpenDelim(::syntax::parse::token::Paren)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::BinOp(::syntax::parse::token::And)));
				tt.extend(::quasi::ToTokens::to_tokens(&output_type_id, ext_cx).into_iter());
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::OpenDelim(::syntax::parse::token::Brace)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("payload"), ::syntax::parse::token::Plain)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Colon));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("self"), ::syntax::parse::token::Plain)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Dot));
				tt.extend(::quasi::ToTokens::to_tokens(&function_name, ext_cx).into_iter());
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::OpenDelim(::syntax::parse::token::Paren)));

				for arg_expr in input_args_exprs {
					tt.extend(::quasi::ToTokens::to_tokens(&arg_expr, ext_cx).into_iter());
					tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Comma));
				}

				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::CloseDelim(::syntax::parse::token::Paren)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::CloseDelim(::syntax::parse::token::Brace)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Comma));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::ModSep));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("bincode"), ::syntax::parse::token::ModName)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::ModSep));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("SizeLimit"), ::syntax::parse::token::ModName)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::ModSep));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("Infinite"), ::syntax::parse::token::Plain)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::CloseDelim(::syntax::parse::token::Paren)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Dot));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::Ident(ext_cx.ident_of("unwrap"), ::syntax::parse::token::Plain)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::OpenDelim(::syntax::parse::token::Paren)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::CloseDelim(::syntax::parse::token::Paren)));
				tt.push(::syntax::ast::TokenTree::Token(_sp, ::syntax::parse::token::CloseDelim(::syntax::parse::token::Brace)));
				tt
			}))
	};
	quote_expr!(cx, {
		let input: $input_type_id = $deserialize_expr;
		$invoke_serialize_stmt
	})

}

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

	let mut dispatch_table = Vec::new();
	for impl_item in impl_items {
		if let ImplItemKind::Method(ref signature, ref block) = impl_item.node {
			dispatch_table.push(push_invoke_signature_aster(cx, builder, item, &impl_item, signature, push));
		}
	}

	let dispatch_expr_1 = implement_dispatch_arm(cx, builder, &dispatch_table[1]);

    Ok(quote_item!(cx,
        impl $impl_generics ::codegen::interface::IpcInterface<$ty> for $ty $where_clause {
			fn dispatch<R>(&self, r: &mut R) -> Vec<u8>
				where R: ::std::io::Read
            {
				let mut method_num = vec![0u8;2];
				match r.read(&mut method_num) {
					Ok(size) if size == 0 => return vec![],
					Err(e) => { panic!("ipc read error, aborting"); }
					_ => {}
				}
				match method_num[0] + method_num[1]*256 {
					0 => {
						$dispatch_expr_1
					}
					_ => vec![]
				}
			}
			fn invoke<W>(&self, method_num: u16, payload: &Option<Vec<u8>>, w: &mut W) -> Vec<u8>
				where W: ::std::io::Write
			{
				vec![]
			}

		}
    ).unwrap())
}
