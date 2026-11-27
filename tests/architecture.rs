//! Bounded syntax gate. Supported attributes: cfg, derive(standard/Serde/Clap),
//! serde, command, arg, and standard lint/doc attributes. Paths encoded in
//! attribute strings are checked; unknown attributes and generated source fail.
use std::fs;
use std::path::{Path, PathBuf};
use syn::parse::Parser;
use syn::visit::{self, Visit};
use syn::{Attribute, Item, ItemUse, Path as SynPath, UseTree};

const OWNERS: &[&str] = &["store", "adapters", "analysis", "model", "app"];

struct Gate {
    owner: String,
    depth: usize,
    source: Option<PathBuf>,
    errors: Vec<String>,
    modules: Vec<Vec<String>>,
}
impl Gate {
    fn error(&mut self, text: impl Into<String>) {
        self.errors.push(text.into());
    }
    fn path(&mut self, segments: &[String], glob: bool) {
        if segments.is_empty() {
            return;
        }
        if segments[0] == "self" && segments.len() > 1 {
            if self
                .modules
                .last()
                .is_some_and(|modules| modules.contains(&segments[1]))
            {
                return;
            }
            self.path(&segments[1..], glob);
            return;
        }
        let first = segments[0].as_str();
        if first == "crate" || first == "mgb_ops" {
            if (first == "crate") == (self.owner == "interface") {
                self.error("wrong library path prefix");
                return;
            }
            if segments.len() < 2 || !OWNERS.contains(&segments[1].as_str()) {
                if self.owner == "interface"
                    && segments.len() == 3
                    && segments[1] == "BatchStatus"
                    && ["Completed", "Failed"].contains(&segments[2].as_str())
                {
                    return;
                }
                if self.owner == "interface"
                    && segments.len() == 3
                    && segments[1] == "OperationControl"
                    && segments[2] == "default"
                {
                    return;
                }
                if self.owner == "interface"
                    && segments.len() == 3
                    && segments[1] == "Draft"
                    && ["new", "open", "import_toml", "import_manifest"]
                        .contains(&segments[2].as_str())
                {
                    return;
                }
                if self.owner == "interface"
                    && segments.len() == 3
                    && segments[1] == "DraftAction"
                    && ["Preparation", "ObservationPreview", "ForecastPreview"]
                        .contains(&segments[2].as_str())
                {
                    return;
                }
                // The facade exposes only app APIs to interfaces.
                if self.owner == "interface"
                    && segments.len() == 2
                    && [
                        "SeriesQuery",
                        "query_series",
                        "export_series_csv",
                        "RegistrationSelection",
                        "RegistrationMapping",
                        "normalize_forecast",
                        "import_level_references",
                        "NormalizeForecastRequest",
                        "NormalizeForecastReport",
                        "NormalizeForecastError",
                        "prepare",
                        "prepare_controlled",
                        "execute_prepared_controlled",
                        "OperationControl",
                        "run_draft",
                        "Draft",
                        "execute_prepared",
                        "PreparationRequest",
                        "PreparationReport",
                        "ExecutionRequest",
                        "ExecutionReport",
                        "OperationError",
                        "ScenarioExecution",
                        "BatchStatus",
                        "inspect",
                        "example_config",
                        "InspectionRequest",
                        "InspectionReport",
                        "FindingStatus",
                        "FindingCode",
                    ]
                    .contains(&segments[1].as_str())
                {
                    return;
                }
                self.error("library root bypass or unknown owner");
                return;
            }
            let target = segments[1].as_str();
            let allowed = target == self.owner
                || self.owner == "interface"
                || self.owner == "app"
                || (target == "store"
                    && ["model", "analysis", "adapters"].contains(&self.owner.as_str()));
            if !allowed || (glob && target != self.owner) {
                self.error(format!("forbidden edge {} -> {target}", self.owner));
            }
        } else if first == "super" {
            let count = segments.iter().take_while(|s| *s == "super").count();
            if count > self.depth {
                self.error("relative path escapes owner");
            }
        } else if OWNERS.contains(&first) && first != self.owner {
            self.error("cross-owner paths must be explicit");
        }
    }
    fn use_tree(&mut self, tree: &UseTree, prefix: &mut Vec<String>, public: bool) {
        match tree {
            UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.use_tree(&p.tree, prefix, public);
                prefix.pop();
            }
            UseTree::Name(n) => {
                prefix.push(n.ident.to_string());
                self.check_use(prefix, false, public);
                prefix.pop();
            }
            UseTree::Rename(n) => {
                prefix.push(n.ident.to_string());
                self.check_use(prefix, false, public);
                prefix.pop();
            }
            UseTree::Glob(_) => self.check_use(prefix, true, public),
            UseTree::Group(g) => {
                for t in &g.items {
                    self.use_tree(t, prefix, public);
                }
            }
        }
    }
    fn check_use(&mut self, prefix: &[String], glob: bool, public: bool) {
        self.path(prefix, glob);
        if public
            && !(prefix.first().is_some_and(|p| p == "self")
                && prefix.len() >= 3
                && self
                    .modules
                    .last()
                    .is_some_and(|modules| modules.contains(&prefix[1])))
        {
            self.error("re-exports must use self::private_module::item");
        }
    }
    fn token_paths(&mut self, text: &str) {
        // Token display separates punctuation; strings are parsed separately below.
        let mut words = Vec::new();
        for part in text.split_whitespace() {
            if part == ":" {
                continue;
            }
            let clean = part.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if !clean.is_empty() {
                words.push(clean.to_string());
            }
        }
        for i in 0..words.len() {
            if ["crate", "mgb_ops", "super"].contains(&words[i].as_str()) {
                self.path(&words[i..words.len().min(i + 3)], false);
            }
        }
    }
}
// Handle the JSON literal grammar and visit every embedded Rust expression.
fn json_value(input: syn::parse::ParseStream<'_>, gate: &mut Gate) -> syn::Result<()> {
    if input.peek(syn::token::Brace) {
        let fields;
        syn::braced!(fields in input);
        while !fields.is_empty() {
            fields.parse::<syn::LitStr>()?;
            fields.parse::<syn::Token![:]>()?;
            json_value(&fields, gate)?;
            if !fields.is_empty() {
                fields.parse::<syn::Token![,]>()?;
            }
        }
    } else if input.peek(syn::token::Bracket) {
        let items;
        syn::bracketed!(items in input);
        while !items.is_empty() {
            json_value(&items, gate)?;
            if !items.is_empty() {
                items.parse::<syn::Token![,]>()?;
            }
        }
    } else {
        gate.visit_expr(&input.parse::<syn::Expr>()?);
    }
    Ok(())
}
impl<'ast> Visit<'ast> for Gate {
    fn visit_visibility(&mut self, _: &'ast syn::Visibility) {}
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        // A bare expression identifier names a value, including local variables.
        if path.qself.is_none() && path.path.get_ident().is_some() {
            for attribute in &path.attrs {
                self.visit_attribute(attribute);
            }
            return;
        }
        visit::visit_expr_path(self, path);
    }
    fn visit_path(&mut self, path: &'ast SynPath) {
        self.path(
            &path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>(),
            false,
        );
        visit::visit_path(self, path);
    }
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        self.use_tree(
            &item.tree,
            &mut Vec::new(),
            !matches!(item.vis, syn::Visibility::Inherited),
        );
    }
    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        for attr in &item.attrs {
            self.visit_attribute(attr);
        }
        if let Some((_, items)) = &item.content {
            self.depth += 1;
            self.modules.push(private_modules(items));
            for item in items {
                self.visit_item(item);
            }
            self.modules.pop();
            self.depth -= 1;
        }
    }
    fn visit_item_extern_crate(&mut self, _: &'ast syn::ItemExternCrate) {
        self.error("extern crate is unsupported");
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.visit_path(&mac.path);
        let name = mac.path.segments.last().unwrap().ident.to_string();
        if name == "json" {
            let path = mac
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>();
            if !["app", "store"].contains(&self.owner.as_str()) || path != ["serde_json", "json"] {
                self.error("JSON literals require serde_json::json in app or store");
            }
            let parse = |input: syn::parse::ParseStream<'_>| json_value(input, self);
            if parse.parse2(mac.tokens.clone()).is_err() {
                self.error("unsupported JSON literal syntax");
            }
            return;
        }
        if name == "env" {
            if !mac.path.is_ident("env")
                || self.source.as_deref() != Some(Path::new("app/assistant.rs"))
                || !syn::parse2::<syn::LitStr>(mac.tokens.clone())
                    .is_ok_and(|literal| literal.value() == "CARGO_PKG_VERSION")
            {
                self.error("env permits only the assistant's Cargo package version");
            }
            return;
        }
        if name == "include_bytes" {
            let asset = syn::parse2::<syn::LitStr>(mac.tokens.clone());
            let approved = match (self.owner.as_str(), self.source.as_deref()) {
                ("analysis", Some(path)) if path == Path::new("analysis/report_graphics.rs") => {
                    vec!["../../assets/typography/NotoSans-Regular.ttf".to_owned()]
                }
                ("interface", Some(path)) if path == Path::new("bin/desktop/theme.rs") => {
                    vec!["../../../assets/typography/NotoSans-Regular.ttf".to_owned()]
                }
                ("interface", Some(path)) if path == Path::new("bin/desktop/icons.rs") => {
                    let mut assets: Vec<_> = [
                        "folder", "recent", "settings", "close", "import", "refresh", "map",
                        "charts", "data", "files", "chat", "review", "analysis", "bulletin",
                        "bundle", "jobs", "reset", "run", "acquire", "geometry", "soil",
                        "language", "light", "dark",
                    ]
                    .iter()
                    .map(|name| format!("../../../assets/icons/{name}.png"))
                    .collect();
                    assets.extend(
                        [32, 48, 64, 128, 256]
                            .map(|size| format!("../../../assets/branding/hge-{size}.png")),
                    );
                    assets
                }
                _ => Vec::new(),
            };
            let valid = mac.path.is_ident("include_bytes")
                && asset.is_ok_and(|literal| approved.contains(&literal.value()));
            if !valid {
                self.error(
                    "include_bytes permits only approved static assets at their designated uses",
                );
            }
            return;
        }
        if name == "include_str" {
            let asset = syn::parse2::<syn::LitStr>(mac.tokens.clone());
            let valid = mac.path.is_ident("include_str")
                && asset.is_ok_and(|literal| {
                    self.owner == "interface"
                        && self.source.as_deref() == Some(Path::new("bin/desktop/icons.rs"))
                        && literal.value() == "../../../assets/desktop/br.ufrgs.hge.MgbOps.desktop"
                });
            if !valid {
                self.error("include_str permits only approved static text at its designated use");
            }
            return;
        }
        if ![
            "format",
            "format_args",
            "write",
            "writeln",
            "eprintln",
            "println",
            "vec",
            "matches",
            "assert",
            "assert_eq",
            "assert_ne",
            "unreachable",
            "panic",
            "cfg",
        ]
        .contains(&name.as_str())
        {
            self.error(format!("unsupported macro {name}"));
        }
        let tokens = mac.tokens.to_string();
        if contains_bang(mac.tokens.clone()) {
            self.error("nested macros are unsupported");
        }
        self.token_paths(&tokens);
    }
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        self.visit_path(attr.path());
        if attr.path().get_ident().is_none() {
            self.error("qualified attribute is unsupported");
            return;
        }
        let name = attr.path().segments.last().unwrap().ident.to_string();
        if name == "default" && matches!(attr.meta, syn::Meta::Path(_)) {
            return;
        }
        if let syn::Meta::NameValue(value) = &attr.meta
            && !matches!(
                &value.value,
                syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(_),
                    ..
                })
            )
        {
            self.error("generated attribute value is unsupported");
            return;
        }
        if ![
            "derive", "serde", "command", "arg", "cfg", "cfg_attr", "doc", "allow", "warn", "deny",
            "forbid", "test", "ignore",
        ]
        .contains(&name.as_str())
        {
            self.error(format!("unsupported attribute {name}"));
            return;
        }
        if name == "cfg_attr" {
            self.error("cfg_attr is unsupported");
            return;
        }
        if let syn::Meta::List(list) = &attr.meta {
            let text = list.tokens.to_string();
            if contains_bang(list.tokens.clone()) {
                self.error("generated attribute is unsupported");
                return;
            }
            self.token_paths(&text);
            if name == "derive" {
                for derive in text.split(',').map(str::trim) {
                    if ![
                        "Debug",
                        "Clone",
                        "Copy",
                        "Default",
                        "PartialEq",
                        "Eq",
                        "PartialOrd",
                        "Ord",
                        "Serialize",
                        "Deserialize",
                        "Parser",
                        "Args",
                        "Subcommand",
                        "ValueEnum",
                    ]
                    .contains(&derive)
                    {
                        self.error(format!("unsupported derive {derive}"));
                    }
                }
            }
            if ["serde", "command", "arg"].contains(&name.as_str()) {
                let result = attr.parse_nested_meta(|meta| {
                    let key = meta
                        .path
                        .get_ident()
                        .map(ToString::to_string)
                        .unwrap_or_default();
                    if name == "arg" && ["requires", "conflicts_with"].contains(&key.as_str()) {
                        let literal: syn::LitStr = meta.value()?.parse()?;
                        let argument = literal.value();
                        if argument.is_empty()
                            || !argument.bytes().all(|byte| {
                                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
                            })
                        {
                            return Err(meta.error("requires must name another argument"));
                        }
                        return Ok(());
                    }
                    let path_keys = [
                        "default",
                        "default_value",
                        "with",
                        "serialize_with",
                        "skip_serializing_if",
                        "deserialize_with",
                        "from",
                        "try_from",
                        "into",
                        "value_parser",
                        "value_enum",
                    ];
                    let plain_keys = [
                        "deny_unknown_fields",
                        "alias",
                        "rename",
                        "rename_all",
                        "skip",
                        "skip_serializing",
                        "skip_deserializing",
                        "flatten",
                        "transparent",
                        "tag",
                        "content",
                        "untagged",
                        "long",
                        "global",
                        "short",
                        "version",
                        "subcommand",
                        "help",
                        "about",
                    ];
                    if !path_keys.contains(&key.as_str()) && !plain_keys.contains(&key.as_str()) {
                        return Err(meta.error("unsupported helper attribute"));
                    }
                    if meta.input.peek(syn::Token![=]) && key == "global" {
                        let _: syn::LitBool = meta.value()?.parse()?;
                    } else if meta.input.peek(syn::Token![=]) {
                        let literal: syn::LitStr = meta.value()?.parse()?;
                        if path_keys.contains(&key.as_str()) {
                            let path: SynPath = literal.parse()?;
                            self.visit_path(&path);
                        }
                    } else if !meta.input.is_empty() && !meta.input.peek(syn::Token![,]) {
                        return Err(meta.error("unsupported helper syntax"));
                    }
                    Ok(())
                });
                if result.is_err() {
                    self.error("unsupported helper attribute");
                }
            }
        }
    }
}
fn contains_bang(tokens: proc_macro2::TokenStream) -> bool {
    tokens.into_iter().any(|token| match token {
        proc_macro2::TokenTree::Punct(p) => p.as_char() == '!',
        proc_macro2::TokenTree::Group(g) => contains_bang(g.stream()),
        _ => false,
    })
}
fn private_modules(items: &[Item]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(m) if matches!(m.vis, syn::Visibility::Inherited) => {
                Some(m.ident.to_string())
            }
            _ => None,
        })
        .collect()
}
fn check_at(owner: &str, source: &str, depth: usize) -> Vec<String> {
    check_source(owner, source, depth, None)
}
fn check_source(owner: &str, source: &str, depth: usize, path: Option<&Path>) -> Vec<String> {
    let file = syn::parse_file(source).expect("source parses");
    let mut gate = Gate {
        owner: owner.into(),
        depth,
        source: path.map(Path::to_path_buf),
        errors: Vec::new(),
        modules: vec![private_modules(&file.items)],
    };
    gate.visit_file(&file);
    gate.errors
}
fn check(owner: &str, source: &str) -> Vec<String> {
    check_at(owner, source, 0)
}
fn root(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("root parses");
    let mut errors = Vec::new();
    let valid_attrs = |attrs: &[Attribute]| {
        attrs.iter().all(|a| {
            ["doc", "allow", "warn", "deny", "forbid"]
                .iter()
                .any(|n| a.path().is_ident(n))
                && match &a.meta {
                    syn::Meta::NameValue(v) => matches!(
                        &v.value,
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(_),
                            ..
                        })
                    ),
                    syn::Meta::List(v) => !contains_bang(v.tokens.clone()),
                    _ => true,
                }
        })
    };
    if !valid_attrs(&file.attrs) {
        errors.push("unsupported root attribute".into());
    }
    for item in file.items {
        match item {
            Item::Mod(m)
                if OWNERS.contains(&m.ident.to_string().as_str())
                    && m.content.is_none()
                    && valid_attrs(&m.attrs) => {}
            Item::Use(u) => {
                fn no_glob(t: &UseTree) -> bool {
                    match t {
                        UseTree::Glob(_) => false,
                        UseTree::Path(p) => no_glob(&p.tree),
                        UseTree::Group(g) => g.items.iter().all(no_glob),
                        _ => true,
                    }
                }
                fn origin(t: &UseTree) -> bool {
                    match t {
                        UseTree::Path(p) => {
                            OWNERS.contains(&p.ident.to_string().as_str()) && no_glob(&p.tree)
                        }
                        _ => false,
                    }
                }
                if !valid_attrs(&u.attrs)
                    || !matches!(u.vis, syn::Visibility::Public(_))
                    || !origin(&u.tree)
                {
                    errors.push("invalid root re-export".into());
                }
            }
            _ => errors.push("root permits only owner modules and re-exports".into()),
        }
    }
    errors
}
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}
#[test]
fn owned_sources_obey_graph() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    sources(&dir, &mut files);
    assert!(files.contains(&dir.join("lib.rs")) && files.contains(&dir.join("bin/cli/main.rs")));
    for file in files {
        let relative = file.strip_prefix(&dir).unwrap();
        let first = relative
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_str()
            .unwrap();
        let source = fs::read_to_string(&file).unwrap();
        let errors = if first == "lib.rs" {
            root(&source)
        } else {
            let owner = if first == "main.rs" || first == "bin" {
                "interface"
            } else {
                first.trim_end_matches(".rs")
            };
            assert!(
                owner == "interface" || OWNERS.contains(&owner),
                "unchecked source: {}",
                file.display()
            );
            let depth = if first.ends_with(".rs") {
                0
            } else {
                relative.components().count()
                    - 1
                    - usize::from(relative.file_name().is_some_and(|n| n == "mod.rs"))
            };
            check_source(owner, &source, depth, Some(relative))
        };
        assert!(errors.is_empty(), "{}: {errors:?}", file.display());
    }
}
#[test]
fn graph_fixtures() {
    for (owner, target) in [
        ("adapters", "store"),
        ("analysis", "store"),
        ("model", "store"),
        ("app", "store"),
        ("app", "adapters"),
        ("app", "analysis"),
        ("app", "model"),
    ] {
        assert!(check(owner, &format!("use crate::{target}::Thing;")).is_empty());
    }
    for (owner, target) in [
        ("store", "model"),
        ("model", "analysis"),
        ("analysis", "adapters"),
        ("adapters", "analysis"),
        ("model", "adapters"),
        ("adapters", "model"),
        ("model", "app"),
        ("app", "interface"),
    ] {
        assert!(!check(owner, &format!("use crate::{target}::Thing;")).is_empty());
    }
    for source in [
        "#[crate::model::serde(default)] struct S;",
        "use crate::store::Thing; pub use self::Thing as Alias;",
        "use crate::store::Thing; pub use crate::adapters::Thing as Alias;",
        "#[doc = include_str!(\"outside.rs\")] struct S;",
        "use crate::store as inner; pub use self::inner::Thing;",
        "mod model { use crate::model::Thing; } pub use self::model::Thing;",
        "fn x() { println!(\"!\", include_str!(\"outside\")); }",
        "use crate::{model::Thing as Alias};",
        "mod x { use super::super::model::Thing; }",
        "#[cfg(test)] mod x { use crate::app::Thing; }",
        "use crate::model::*;",
        "extern crate self as alias;",
        "include!(\"file.rs\");",
        "custom_macro!(crate::model::x);",
        "format!(\"{}\", include_str!(\"outside\"));",
        "mod x { use self::super::super::model::Thing; }",
        "#[serde(default = \"crate::model::default_value\")] struct S;",
        "pub use crate::store::Thing;",
        "crate::model::println!(\"x\");",
        "#[serde(with = \"model\")] struct S;",
        "#[serde(skip_serializing_if = \"crate::model::hidden\")] struct S;",
        "#[path = \"outside.rs\"] use crate::store::Thing;",
    ] {
        assert!(!check("adapters", source).is_empty(), "accepted {source}");
    }
    for source in [
        "mod inner { pub struct Thing; } pub use self::inner::Thing;",
        "mod model { pub struct Thing; } pub use self::model::Thing;",
        "fn x() { println!(\"hello!\"); }",
        "#[allow(clippy::todo, reason = \"justified!\")] fn x() {}",
        "use crate::store::{Thing as Alias, Other};",
        "mod x { use super::helper; }",
        "#[serde(default = \"crate::store::default_value\")] struct S;",
        "#[serde(skip_serializing_if = \"Option::is_none\")] struct S;",
        "#[serde(default = \"crate::adapters::default_value\")] struct S;",
    ] {
        assert!(check("adapters", source).is_empty(), "rejected {source}");
    }
    assert!(!root("#![doc(include = include_str!(\"outside.rs\"))] mod app;").is_empty());
    assert!(!root("#![doc = include_str!(\"outside.rs\")] mod app;").is_empty());
    assert!(!root("#[doc = include_str!(\"outside.rs\")] mod app;").is_empty());
    assert!(!root("#[path = \"outside.rs\"] mod app;").is_empty());
    assert!(check_at("app", "use super::helper;", 1).is_empty());
    assert!(!root("fn helper() {} ").is_empty());
    assert!(!root("pub use model::*;").is_empty());
    assert!(!root("pub use model::{x::*};").is_empty());
    assert!(root("pub mod app; pub use app::{inspect, InspectionRequest};").is_empty());
    assert!(check("interface", "use mgb_ops::app::inspect;").is_empty());
    assert!(
        check(
            "interface",
            "use mgb_ops::{normalize_forecast, NormalizeForecastRequest};"
        )
        .is_empty()
    );
    assert!(check("interface", "use mgb_ops::{prepare, execute_prepared, PreparationRequest, ExecutionRequest, BatchStatus};").is_empty());
    assert!(
        check(
            "interface",
            "fn x() { let status = mgb_ops::BatchStatus::Completed; }"
        )
        .is_empty()
    );
    assert!(!check("interface", "use mgb_ops::OperationError::unknown;").is_empty());
    assert!(
        check(
            "app",
            "mod forecast { pub struct Thing; } pub use self::forecast::Thing;"
        )
        .is_empty()
    );
}

#[test]
fn removed_sql_storage_macros_are_rejected() {
    for source in [
        r#"const DDL: &str = include_str!("sql/observations.sql");"#,
        "fn x() { rusqlite::params![row.id, None::<f64>]; }",
    ] {
        assert!(!check("store", source).is_empty());
    }
}

#[test]
fn owner_names_as_local_values_do_not_create_import_edges() {
    assert!(check("store", "fn x(model: &str) { let _ = model.len(); }").is_empty());
    assert!(!check("store", "use model;").is_empty());
    assert!(!check("store", "fn x() { model::run(); }").is_empty());
    assert!(!check("store", "fn x() { crate::model::run(); }").is_empty());
}

#[test]
fn clap_requires_takes_a_literal_argument_name() {
    assert!(
        check(
            "interface",
            r#"struct Args { #[arg(long, requires = "time_zone")] reference: String }"#
        )
        .is_empty()
    );
    assert!(
        !check(
            "interface",
            r#"struct Args { #[arg(requires = "crate::store::thing")] reference: String }"#
        )
        .is_empty()
    );
    assert!(
        !check(
            "interface",
            r#"struct Args { #[arg(requires = key)] reference: String }"#
        )
        .is_empty()
    );
}

#[test]
fn nested_interfaces_keep_relative_paths_inside_the_interface() {
    assert!(check_at("interface", "use super::data::Job;", 1).is_empty());
    let escaped = check_at("interface", "use super::super::data::Job;", 1);
    assert!(escaped.iter().any(|e| e.contains("escapes owner")));
    let bypass = check_at("interface", "use crate::store::ObservationStore;", 1);
    assert!(
        bypass
            .iter()
            .any(|e| e.contains("wrong library path prefix"))
    );
    assert!(
        check(
            "interface",
            "#[test] #[ignore = \"requires a graphics adapter\"] fn graphics() {}"
        )
        .is_empty()
    );
}

#[test]
fn shared_font_inclusion_is_limited_to_its_static_paths() {
    let source = Path::new("analysis/report_graphics.rs");
    let accepted =
        r#"const FONT: &[u8] = include_bytes!("../../assets/typography/NotoSans-Regular.ttf");"#;
    assert!(check_source("analysis", accepted, 1, Some(source)).is_empty());
    assert!(!check_source("store", accepted, 1, Some(source)).is_empty());
    assert!(
        !check_source(
            "analysis",
            accepted,
            1,
            Some(Path::new("analysis/other.rs"))
        )
        .is_empty()
    );
    let escaped = r#"const FONT: &[u8] = include_bytes!("../assets/NotoSans-Regular.ttf");"#;
    assert!(!check_source("analysis", escaped, 1, Some(source)).is_empty());
    let desktop =
        r#"const FONT: &[u8] = include_bytes!("../../../assets/typography/NotoSans-Regular.ttf");"#;
    let theme = Path::new("bin/desktop/theme.rs");
    assert!(check_source("interface", desktop, 2, Some(theme)).is_empty());
    assert!(!check_source("interface", accepted, 2, Some(theme)).is_empty());
    assert!(
        !check_source(
            "interface",
            desktop,
            2,
            Some(Path::new("bin/desktop/mod.rs"))
        )
        .is_empty()
    );
}

#[test]
fn desktop_images_are_limited_to_named_assets_in_the_icon_module() {
    let source = Path::new("bin/desktop/icons.rs");
    let accepted = r#"const ICON: &[u8] = include_bytes!("../../../assets/icons/map.png");"#;
    assert!(check_source("interface", accepted, 2, Some(source)).is_empty());
    assert!(!check_source("store", accepted, 2, Some(source)).is_empty());
    assert!(
        !check_source(
            "interface",
            accepted,
            2,
            Some(Path::new("bin/desktop/mod.rs"))
        )
        .is_empty()
    );
    for rejected in [
        r#"const ICON: &[u8] = include_bytes!("../../../src/store.rs");"#,
        r#"const ICON: &[u8] = include_bytes!("../../../assets/icons/unknown.png");"#,
        r#"const ICON: &[u8] = include_bytes!(concat!("../../../assets/icons/", "map.png"));"#,
        r#"const ICON: &[u8] = other::include_bytes!("../../../assets/icons/map.png");"#,
    ] {
        assert!(!check_source("interface", rejected, 2, Some(source)).is_empty());
    }
}

#[test]
fn launcher_template_is_limited_to_the_icon_module() {
    let template = r#"const ENTRY: &str = include_str!("../../../assets/desktop/br.ufrgs.hge.MgbOps.desktop");"#;
    let source = Path::new("bin/desktop/icons.rs");
    assert!(check_source("interface", template, 2, Some(source)).is_empty());
    assert!(!check_source("store", template, 2, Some(source)).is_empty());
    assert!(
        !check_source(
            "interface",
            template,
            2,
            Some(Path::new("bin/desktop/platform.rs"))
        )
        .is_empty()
    );
    let outside = r#"const ENTRY: &str = include_str!("../../../src/store.rs");"#;
    assert!(!check_source("interface", outside, 2, Some(source)).is_empty());
}

#[test]
fn json_literals_check_embedded_rust_paths() {
    assert!(check("store", r#"fn f() { serde_json::json!({"key": 1}); }"#).is_empty());
    assert!(
        !check(
            "store",
            r#"fn f() { serde_json::json!({"key": crate::app::thing()}); }"#
        )
        .is_empty()
    );
    assert!(
        check(
            "app",
            r#"fn f() { serde_json::json!({"key":[1, read(crate::store::thing())]}); }"#
        )
        .is_empty()
    );
    assert!(
        !check(
            "app",
            r#"fn f() { serde_json::json!({"key": crate::outside::private()}); }"#
        )
        .is_empty()
    );
    assert!(!check("app", r#"fn f() { serde_json::json!({key: 1}); }"#).is_empty());
    assert!(
        !check(
            "app",
            r#"fn f() { serde_json::json!({"key": include!("generated.rs")}); }"#
        )
        .is_empty()
    );
    assert!(!check("app", r#"fn f() { json!({"key":1}); }"#).is_empty());
}

// Cargo.lock includes optional dependencies. Qualification is allowed only in its
// named test feature; these checks do not replace a resolved-feature/source audit.
// SQLite is an owned-store implementation detail. Defaults remain disabled;
// no native SQLite or legacy Parquet graph is accepted by this source gate.
#[test]
fn registered_storage_dependency_policy() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.toml")).unwrap()).unwrap();
    let dependency = &manifest["dependencies"]["turso"];
    assert_eq!(dependency["version"].as_str(), Some("=0.9.0-pre.6"));
    assert_eq!(dependency["default-features"].as_bool(), Some(false));
    assert!(dependency.get("optional").is_none());
    assert!(dependency.get("features").is_none());
    assert!(dependency.get("path").is_none());
    assert!(dependency.get("git").is_none());
    assert!(manifest["dependencies"].get("parquet").is_none());
    let lock: toml::Value =
        toml::from_str(&fs::read_to_string(root.join("Cargo.lock")).unwrap()).unwrap();
    for package in lock["package"].as_array().unwrap() {
        let name = package["name"].as_str().unwrap();
        assert!(
            !["libsqlite3-sys", "rusqlite", "parquet", "zstd-sys"].contains(&name),
            "unapproved persistence dependency {name}"
        );
        if name.starts_with("turso") {
            assert_eq!(package["version"].as_str(), Some("0.9.0-pre.6"));
            assert!(
                [
                    "turso",
                    "turso_core",
                    "turso_ext",
                    "turso_macros",
                    "turso_parser",
                    "turso_sdk_kit",
                    "turso_sdk_kit_macros",
                    "turso_sqlite"
                ]
                .contains(&name)
            );
        }
    }
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    for path in files {
        if !path.starts_with(root.join("src/store")) {
            let source = fs::read_to_string(&path).unwrap();
            assert!(
                !source.contains("turso::"),
                "database API outside store: {}",
                path.display()
            );
        }
    }
}
