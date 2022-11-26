use rocket_dyn_templates::handlebars::{Context, Handlebars, handlebars_helper, Helper, Output, Renderable, RenderContext};
use serde_json::Value;
use crate::handlebars::{HelperResult, RenderError};
use crate::handlebars::template::TemplateElement;


pub fn modulo(
	h: &Helper<'_, '_>,
	_: &Handlebars,
	_: &Context,
	_: &mut RenderContext<'_, '_>,
	out: &mut dyn Output
) -> HelperResult {
	let index = h.param(0).unwrap().value().as_i64().unwrap();
	let modu = h.param(1).unwrap().value().as_i64().unwrap();
	if (index % modu)==0{
		for h in &h.template().unwrap().elements {
			match h{
				TemplateElement::RawString(s) => {
					out.write(s);
				}
				_=>{}
			}
		}
	}else{
		for h in &h.inverse().unwrap().elements {
			match h{
				TemplateElement::RawString(s) => {
					out.write(s);
				}
				_=>{}
			}
		}
	}

	Ok(())
}

