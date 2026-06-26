use xf_typegen::php::{class_const, finder_stub_ident, fqcn, ident, single_quoted, with_nullable};

#[test]
fn idents() {
    assert_eq!(ident("XF:User"), "XF_User");
    assert_eq!(ident("Vendor\\AddOn:Foo"), "Vendor_AddOn_Foo");
    assert_eq!(finder_stub_ident("XF:User"), "Finder_XF_User");
}

#[test]
fn fqcns() {
    assert_eq!(fqcn("XF\\Entity\\User"), "\\XF\\Entity\\User");
    assert_eq!(fqcn("\\XF\\Entity\\User"), "\\XF\\Entity\\User");
    assert_eq!(class_const("XF\\Entity\\User"), "\\XF\\Entity\\User::class");
}

#[test]
fn nullable() {
    assert_eq!(with_nullable("int", true), "int|null");
    assert_eq!(with_nullable("int", false), "int");
    assert_eq!(with_nullable("array|null", true), "array|null");
    assert_eq!(with_nullable("array|bool", true), "array|bool|null");
}

#[test]
fn quoting() {
    assert_eq!(single_quoted("XF:User"), "'XF:User'");
    assert_eq!(single_quoted("Vendor\\AddOn:Foo"), "'Vendor\\\\AddOn:Foo'");
}
