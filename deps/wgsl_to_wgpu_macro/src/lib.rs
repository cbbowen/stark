extern crate proc_macro;
use quote::quote;
use wgsl_to_wgpu::*;

struct ShaderModuleInput {
    // We could avoid needing the current_path if we had either:
    // * `proc_macro_expand`: We could express this as `shader_module!(include_str!(...))`.
    // * `proc_macro_span`: We could get the current path from `Span::source_file`.
    visibility: syn::Visibility,
    _mod: syn::Token![mod],
    wgsl_path: syn::LitStr,
    _in: syn::Token![in],
    current_path: syn::LitStr,
}

impl syn::parse::Parse for ShaderModuleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            visibility: input.parse()?,
            _mod: input.parse()?,
            wgsl_path: input.parse()?,
            _in: input.parse()?,
            current_path: input.parse()?,
        })
    }
}

#[proc_macro]
pub fn shader(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: ShaderModuleInput = syn::parse_macro_input!(input);
    let options = wgsl_to_wgpu::WriteOptions {
        derive_bytemuck_vertex: true,
        derive_encase_host_shareable: true,
        matrix_vector_types: MatrixVectorTypes::Glam,
        ..Default::default()
    };
    let current_path: std::path::PathBuf = input.current_path.value().into();
    let wgsl_path: std::path::PathBuf = input.wgsl_path.value().into();
    let visibility = input.visibility;

    let wgsl_source = std::fs::read_to_string(current_path.join(&wgsl_path)).unwrap();
    let rs_source =
        create_shader_module_tokens(&wgsl_source, Some(&wgsl_path.to_string_lossy()), options)
            .unwrap();

    let name_parts: Vec<_> = wgsl_path
        .with_extension("")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    let mod_name = name_parts.as_slice().join("_");
    let mod_name = syn::Ident::new(&mod_name, input.wgsl_path.span());

    quote! {
        #visibility mod #mod_name {
            #rs_source
        }
    }
    .into()
}
