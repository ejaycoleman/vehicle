use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSourceFuture;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use deno_core::anyhow::Error;
use deno_core::futures::FutureExt;
use deno_core::resolve_import;
use std::path::Path;
use std::pin::Pin;

pub struct TypescriptModuleLoader;

impl ModuleLoader for TypescriptModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _is_main: bool
  ) -> Result<ModuleSpecifier, Error> {
    Ok(resolve_import(specifier, referrer)?)
  }

  fn load(
    &self,
    module_specifier: &ModuleSpecifier,
    _maybe_referrer: Option<ModuleSpecifier>,
    _is_dyn_import: bool
  ) -> Pin<Box<ModuleSourceFuture>> {
    let module_specifier = module_specifier.clone();

    (
      async move {
        let path = module_specifier.as_str();

        let media_type = MediaType::from(Path::new(&path));

        let (module_type, should_transpile) = match MediaType::from(Path::new(&path)) {
          MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
            (ModuleType::JavaScript, false)
          }
          MediaType::Jsx => (ModuleType::JavaScript, true),
          | MediaType::TypeScript
          | MediaType::Mts
          | MediaType::Cts
          | MediaType::Dts
          | MediaType::Dmts
          | MediaType::Dcts
          | MediaType::Tsx => (ModuleType::JavaScript, true),
          MediaType::Json => (ModuleType::Json, false),
          _ => {
            // TODO improve this for efficiency:
            // 'import {} from' can be extensionless - attempt to parse anyway
            // another error will soon be encountered if extensionless files dont contain something valid. 
            (ModuleType::JavaScript, true)
          }
        };

        let code = std::fs::read_to_string(module_specifier.path())?;

        let code = if should_transpile {
          let parsed = deno_ast::parse_module(ParseParams {
            specifier: module_specifier.to_string(),
            text_info: SourceTextInfo::from_string(code),
            media_type,
            capture_tokens: false,
            scope_analysis: false,
            maybe_syntax: None,
          })?;
          parsed.transpile(&Default::default())?.text
        } else {
          code
        };
        let module = ModuleSource {
          code: code.into_bytes().into_boxed_slice(),
          module_type,
          module_url_specified: module_specifier.to_string(),
          module_url_found: module_specifier.to_string(),
        };
        Ok(module)
      }
    ).boxed_local()
  }
}