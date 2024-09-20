#![cfg_attr(feature = "track_path", feature(track_path))]

extern crate proc_macro;

use quote::quote;
use wgsl_to_wgpu::*;

mod keywords {
    syn::custom_keyword!(filterable);
}

struct FilterableOption {
    _filterable: keywords::filterable,
    _colon: syn::Token![:],
    value: syn::LitBool,
}

impl syn::parse::Parse for FilterableOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            _filterable: input.parse()?,
            _colon: input.parse()?,
            value: input.parse()?,
        })
    }
}

impl FilterableOption {
    fn value(&self) -> bool {
        self.value.value
    }
}

#[derive(Default)]
struct MaybeFilterableOption(Option<FilterableOption>);

impl MaybeFilterableOption {
    fn value(&self) -> bool {
        self.0.as_ref().map(FilterableOption::value).unwrap_or(true)
    }
}

impl syn::parse::Parse for MaybeFilterableOption {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let result = if input.peek(keywords::filterable) {
            Some(input.parse()?)
        } else {
            None
        };
        Ok(MaybeFilterableOption(result))
    }
}

#[derive(Default)]
struct ShaderModuleOptions {
    _where: Option<syn::Token![where]>,
    filterable: MaybeFilterableOption,
}

impl syn::parse::Parse for ShaderModuleOptions {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = ShaderModuleOptions::default();
        if input.peek(syn::Token![where]) {
            result._where = input.parse()?;
            result.filterable = input.parse()?;
        }
        Ok(result)
    }
}

struct ShaderModuleInput {
    // We could avoid needing the current_path if we had either:
    // * `proc_macro_expand`: We could express this as `shader_module!(include_str!(...))`.
    // * `proc_macro_span`: We could get the current path from `Span::source_file`.
    visibility: syn::Visibility,
    _mod: syn::Token![mod],
    wgsl_path: syn::LitStr,
    _in: syn::Token![in],
    current_path: syn::LitStr,
    options: ShaderModuleOptions,
}

impl syn::parse::Parse for ShaderModuleInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            visibility: input.parse()?,
            _mod: input.parse()?,
            wgsl_path: input.parse()?,
            _in: input.parse()?,
            current_path: input.parse()?,
            options: input.parse()?,
        })
    }
}

fn read_to_string(path: impl AsRef<std::path::Path>) -> String {
    let path = path.as_ref();

    #[cfg(feature = "track_path")]
    proc_macro::tracked_path::path(path.to_string_lossy());

    std::fs::read_to_string(path).unwrap()
}

fn preprocess_wgsl(current_path: impl AsRef<std::path::Path>, original_source: &str) -> String {
    let current_path = current_path.as_ref();
    let include_re = regex::Regex::new(r#"\<include!\("(?<path>[^"]*)"\)\s*\{\s*\}"#).unwrap();
    let mut include_sources = Vec::new();
    include_sources.push("".to_string());
    for capture in include_re.captures_iter(original_source) {
        let path_match = capture.name("path").unwrap();
        let path = path_match.as_str();
        let include_path = current_path.join(path);
        let include_source = read_to_string(&include_path);
        let include_source = preprocess_wgsl(include_path.parent().unwrap(), &include_source);
        include_sources.push(include_source);
    }
    let mut result = String::new();
    for (include, split) in include_sources
        .iter()
        .zip(include_re.split(original_source))
    {
        result.push_str(include);
        result.push_str(split);
    }
    result
}

#[proc_macro]
pub fn shader(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input: ShaderModuleInput = syn::parse_macro_input!(input);
    let current_path: std::path::PathBuf = input.current_path.value().into();
    let wgsl_path: std::path::PathBuf = input.wgsl_path.value().into();
    let visibility = input.visibility;

    let current_wgsl_path = current_path.join(&wgsl_path);

    let wgsl_source = read_to_string(&current_wgsl_path);
    let wgsl_source = preprocess_wgsl(current_wgsl_path.parent().unwrap(), &wgsl_source);

    let options = wgsl_to_wgpu::WriteOptions {
        derive_bytemuck_vertex: true,
        derive_encase_host_shareable: true,
        matrix_vector_types: MatrixVectorTypes::Glam,
        non_filterable: !input.options.filterable.value(),
        ..Default::default()
    };
    let rs_source = create_shader_module_embedded(&wgsl_source, options).unwrap();

    // We're going more work than strictly necessary here because `wgsl_to_wgpu` internally produces a `TokenStream`, but that's not a big concern.
    let rs_source: proc_macro2::TokenStream = rs_source.parse().unwrap();

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
