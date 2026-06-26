use super::EntityMode;
use crate::contract::{Cardinality, Contract, Entity, Relation};
use crate::php::{collection_stub_ident, finder_stub_ident, fqcn, mixin_target_ident, with_nullable};
use std::collections::BTreeSet;
use std::fmt::Write;

const BASE_FINDER: &str = "\\XF\\Mvc\\Entity\\Finder";
const BASE_ENTITY: &str = "\\XF\\Mvc\\Entity\\Entity";
const COLLECTION: &str = "\\XF\\Mvc\\Entity\\AbstractCollection";
const ARRAY_COLLECTION: &str = "\\XF\\Mvc\\Entity\\ArrayCollection";

pub fn render(contract: &Contract, mode: EntityMode) -> String {
    let mut out = super::banner(
        " *\n * Exclude it from your project's PHP autoload so it is never included at\n * runtime (it re-declares real classes for completion only).\n",
    );

    out.push_str(&helper_stubs(contract));
    out.push('\n');

    match mode {
        EntityMode::Redeclare => {
            for entity in contract.entities.values() {
                out.push_str(&entity_stub(entity));
                out.push('\n');
            }
        }
        EntityMode::Mixin => out.push_str(&mixin_targets(contract)),
    }

    out
}

fn mixin_targets(contract: &Contract) -> String {
    let mut body = String::from("namespace XFIDEHelper {\n");

    for entity in contract.entities.values() {
        let getter_names: BTreeSet<&str> = entity.getters.keys().map(String::as_str).collect();
        let doc = property_doc(&property_sections(entity, &getter_names));

        body.push_str("\t/**\n");
        for line in doc.lines() {
            let _ = write!(body, "\t{}\n", line);
        }
        body.push_str("\t */\n");
        let _ = write!(body, "\tclass {} {{}}\n\n", mixin_target_ident(&entity.short_name));
    }

    body.push_str("}\n");
    body
}

fn helper_stubs(contract: &Contract) -> String {
    let mut body = String::from("namespace XFIDEHelper {\n");
    for entity in contract.entities.values() {
        body.push_str(&collection_stub(entity));
        body.push_str(&finder_stub(entity));
    }
    body.push_str("}\n");
    body
}

fn collection_stub(entity: &Entity) -> String {
    let e = fqcn(&entity.class);
    format!(
        "\t/**\n\
         \t * @method {e}|null first()\n\
         \t * @method {e}|null last()\n\
         \t * @method {e}[] toArray()\n\
         \t * @method \\ArrayIterator<int, {e}> getIterator()\n\
         \t * @method \\XFIDEHelper\\{c} filter(\\Closure $callback)\n\
         \t * @method \\XFIDEHelper\\{c} slice($offset, $length = null, $preserveKeys = true)\n\
         \t * @method \\XFIDEHelper\\{c} reverse($preserveKeys = true)\n\
         \t */\n\
         \tclass {c} extends {ac} {{}}\n\n",
        e = e,
        c = collection_stub_ident(&entity.short_name),
        ac = ARRAY_COLLECTION,
    )
}

fn finder_stub(entity: &Entity) -> String {
    let parent = entity
        .finder
        .as_deref()
        .map(fqcn)
        .unwrap_or_else(|| BASE_FINDER.to_string());
    format!(
        "\t/**\n\
         \t * @method {e}|null fetchOne(?int $offset = null)\n\
         \t * @method \\XFIDEHelper\\{c} fetch(?int $limit = null, ?int $offset = null)\n\
         \t * @method \\XFIDEHelper\\{c} fetchByIds(array $ids)\n\
         \t * @method \\XFIDEHelper\\{s} where($condition, $operator = null, $value = null)\n\
         \t * @method \\XFIDEHelper\\{s} whereOr(array $conditionA, array $conditionB = null)\n\
         \t * @method \\XFIDEHelper\\{s} whereId($id)\n\
         \t * @method \\XFIDEHelper\\{s} whereSql($sql)\n\
         \t * @method \\XFIDEHelper\\{s} with($name, $mustExist = false)\n\
         \t * @method \\XFIDEHelper\\{s} order($field, $direction = 'ASC')\n\
         \t * @method \\XFIDEHelper\\{s} setDefaultOrder($field, $direction = 'ASC')\n\
         \t * @method \\XFIDEHelper\\{s} limit($limit, $offset = null)\n\
         \t * @method \\XFIDEHelper\\{s} limitByPage($page, $perPage, $thisPageExtra = 0)\n\
         \t * @method \\XFIDEHelper\\{s} keyedBy($keyedBy)\n\
         \t */\n\
         \tclass {s} extends {parent} {{}}\n\n",
        e = fqcn(&entity.class),
        c = collection_stub_ident(&entity.short_name),
        s = finder_stub_ident(&entity.short_name),
        parent = parent,
    )
}

fn entity_stub(entity: &Entity) -> String {
    let (namespace, short_class) = split_class(&entity.class);
    let getter_names: BTreeSet<&str> = entity.getters.keys().map(String::as_str).collect();
    let doc = property_doc(&property_sections(entity, &getter_names));

    let mut block = String::new();
    if namespace.is_empty() {
        block.push_str("namespace {\n");
    } else {
        let _ = write!(block, "namespace {} {{\n", namespace);
    }
    block.push_str("\t/**\n");
    for line in doc.lines() {
        let _ = write!(block, "\t{}\n", line);
    }
    block.push_str("\t */\n");
    let _ = write!(block, "\tclass {} extends {} {{}}\n}}\n", short_class, BASE_ENTITY);
    block
}

fn property_sections(entity: &Entity, getters: &BTreeSet<&str>) -> Vec<(&'static str, Vec<String>)> {
    let mut sections = Vec::new();

    if !entity.columns.is_empty() {
        let lines = entity
            .columns
            .iter()
            .map(|(name, col)| {
                property(&with_nullable(&col.php_type, col.nullable), &bypass_name(name, getters))
            })
            .collect();
        sections.push(("COLUMNS", lines));
    }

    if !entity.getters.is_empty() {
        let lines = entity
            .getters
            .iter()
            .map(|(name, getter)| property(&getter.php_type, name))
            .collect();
        sections.push(("GETTERS", lines));
    }

    if !entity.relations.is_empty() {
        let lines = entity
            .relations
            .iter()
            .map(|(name, rel)| property(&relation_type(rel), &bypass_name(name, getters)))
            .collect();
        sections.push(("RELATIONS", lines));
    }

    sections
}

fn property(ty: &str, name: &str) -> String {
    format!("@property {} ${}", ty, name)
}

fn relation_type(rel: &Relation) -> String {
    match rel.to {
        Cardinality::One => match &rel.class {
            Some(class) => format!("{}|null", fqcn(class)),
            None => format!("{}|null", BASE_ENTITY),
        },
        Cardinality::Many => match &rel.class {
            Some(class) => format!("{}|{}[]", COLLECTION, fqcn(class)),
            None => COLLECTION.to_string(),
        },
    }
}

fn property_doc(sections: &[(&str, Vec<String>)]) -> String {
    let mut doc = String::new();
    for (i, (label, lines)) in sections.iter().enumerate() {
        if i > 0 {
            doc.push_str(" *\n");
        }
        let _ = write!(doc, " * {}\n", label);
        for line in lines {
            let _ = write!(doc, " * {}\n", line);
        }
    }
    doc
}

fn bypass_name(name: &str, getters: &BTreeSet<&str>) -> String {
    if getters.contains(name) {
        format!("{}_", name)
    } else {
        name.to_string()
    }
}

fn split_class(class: &str) -> (String, String) {
    let trimmed = class.trim_start_matches('\\');
    match trimmed.rfind('\\') {
        Some(idx) => (trimmed[..idx].to_string(), trimmed[idx + 1..].to_string()),
        None => (String::new(), trimmed.to_string()),
    }
}
