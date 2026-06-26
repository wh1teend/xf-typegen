use crate::contract::Contract;
use crate::php::fqcn;
use std::collections::BTreeMap;
use std::fmt::Write;

pub fn render(contract: &Contract) -> String {
    let mut out = super::banner(
        " *\n * Stubs for XenForo's runtime-generated XFCP class proxies, so the IDE can\n * resolve `class Foo extends XFCP_Foo`. Keep it out of your PHP autoload.\n",
    );

    let mut by_namespace: BTreeMap<&str, Vec<(&str, &str)>> = BTreeMap::new();
    for ext in &contract.class_extensions {
        let (namespace, short) = split(&ext.proxy);
        by_namespace
            .entry(namespace)
            .or_default()
            .push((short, &ext.extends));
    }

    for (namespace, mut classes) in by_namespace {
        classes.sort();

        if namespace.is_empty() {
            out.push_str("namespace {\n");
        } else {
            let _ = write!(out, "namespace {} {{\n", namespace);
        }
        for (short, extends) in classes {
            let _ = write!(out, "\tclass {} extends {} {{}}\n", short, fqcn(extends));
        }
        out.push_str("}\n\n");
    }

    out
}

fn split(class: &str) -> (&str, &str) {
    let trimmed = class.trim_start_matches('\\');
    match trimmed.rfind('\\') {
        Some(idx) => (&trimmed[..idx], &trimmed[idx + 1..]),
        None => ("", trimmed),
    }
}
