pub fn fqcn(class: &str) -> String {
    format!("\\{}", class.trim_start_matches('\\'))
}

pub fn class_const(class: &str) -> String {
    format!("{}::class", fqcn(class))
}

pub fn ident(short: &str) -> String {
    let mut out = String::with_capacity(short.len());
    for ch in short.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

pub fn finder_stub_ident(short: &str) -> String {
    format!("Finder_{}", ident(short))
}

pub fn collection_stub_ident(short: &str) -> String {
    format!("Collection_{}", ident(short))
}

pub fn mixin_target_ident(short: &str) -> String {
    format!("Entity_{}", ident(short))
}

pub fn single_quoted(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{}'", escaped)
}

pub fn with_nullable(php_type: &str, nullable: bool) -> String {
    if nullable && !type_admits_null(php_type) {
        format!("{}|null", php_type)
    } else {
        php_type.to_string()
    }
}

fn type_admits_null(php_type: &str) -> bool {
    php_type
        .split('|')
        .any(|part| part.trim().eq_ignore_ascii_case("null"))
}
