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

	let mut dispath = Dispatch { input_type_name: None, return_type_name: None };

	let inputs = &signature.decl.inputs;
	let input_type_name = if inputs.len() > 0 {
		let first_field_name = field_name(builder, &inputs[0]).name.as_str();
		if  first_field_name == "self" && inputs.len() == 1 { None }
		else {
			let skip = if first_field_name == "self" { 2 } else { 1 };
			let name_str = format!("{}_input", implement.ident.name.as_str());

			let mut tree = builder.item().attr().word("derive(Serialize, Deserialize)").struct_(name_str.as_str())
				.field(format!("{}", field_name(builder, &inputs[skip-1]).name.as_str	())).ty().build(inputs[skip-1].ty.clone());
			for arg in inputs.iter().skip(skip) {
				tree = tree.field(format!("{}", field_name(builder, &arg).name.as_str())).ty().build(arg.ty.clone());
			}

			push(Annotatable::Item(tree.build()));
			Some(name_str.to_owned())
		}
	}
	else {
		None
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
		input_type_name: input_type_name,
		return_type_name: return_type_name,
	}
}

struct Dispatch {
	input_type_name: Option<String>,
	return_type_name: Option<String>,
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

	for impl_item in impl_items {
		if let ImplItemKind::Method(ref signature, ref block) = impl_item.node {
			push_invoke_signature_aster(cx, builder, item, &impl_item, signature, push);
		}
	}

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
						let input: new_input = ::bincode::serde::deserialize_from(r, ::bincode::SizeLimit::Infinite).expect("ipc deserialization error, aborting");
						::bincode::serde::serialize(&new_output { payload: self.new(input.a, input.b) }, ::bincode::SizeLimit::Infinite).unwrap()
					}
					_ => vec![]
				}
            }
			fn invoke<W>(&self, method_num: u16, params: &Option<Vec<u8>>, w: &mut W) -> Vec<u8>
				where W: ::std::io::Write
			{
				vec![]
			}

        }
    ).unwrap())
}
