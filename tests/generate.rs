use xf_typegen::contract::Contract;
use xf_typegen::generate::{render, EntityMode, Target};

const SAMPLE: &str = include_str!("contracts/sample.json");

fn sample() -> Contract {
    Contract::from_str(SAMPLE).expect("sample parses")
}

#[test]
fn sample_parses_with_entities() {
    let c = sample();
    assert!(c.entities.len() > 100, "expected many entities");
    assert!(c.entities.contains_key("XF:User"));
}

#[test]
fn ide_helper_types_user() {
    let php = render(&sample(), Target::IdeHelper, EntityMode::Redeclare).contents;
    assert!(php.contains("class Finder_XF_User extends \\XF\\Finder\\User"));
    assert!(php.contains("@method \\XF\\Entity\\User|null fetchOne(?int $offset = null)"));
    assert!(php.contains("@method \\XFIDEHelper\\Collection_XF_User fetch("));
    assert!(php.contains("@method \\XFIDEHelper\\Finder_XF_User where($condition"));
    assert!(php.contains("@method \\XFIDEHelper\\Finder_XF_User order($field"));
    assert!(php.contains("@method \\XFIDEHelper\\Finder_XF_User keyedBy($keyedBy)"));
    assert!(php.contains("@method \\XFIDEHelper\\Collection_XF_User filter(\\Closure $callback)"));
    assert!(php.contains("class Collection_XF_User extends \\XF\\Mvc\\Entity\\ArrayCollection"));
    assert!(php.contains("@method \\ArrayIterator<int, \\XF\\Entity\\User> getIterator()"));
    assert!(php.contains("namespace XF\\Entity {"));
    assert!(php.contains("class User extends \\XF\\Mvc\\Entity\\Entity"));
    assert!(php.contains("@property string $username"));
    assert!(php.contains(
        "@property \\XF\\Mvc\\Entity\\AbstractCollection|\\XF\\Entity\\UserConnectedAccount[] $ConnectedAccounts"
    ));
}

#[test]
fn mixin_mode_emits_targets_not_redeclarations() {
    let php = render(&sample(), Target::IdeHelper, EntityMode::Mixin).contents;
    assert!(php.contains("class Entity_XF_User {}"));
    assert!(php.contains("@property string $username"));
    assert!(!php.contains("class User extends \\XF\\Mvc\\Entity\\Entity"));
    assert!(!php.contains("namespace XF\\Entity {"));
    assert!(php.contains("class Finder_XF_User extends \\XF\\Finder\\User"));
}

#[test]
fn phpstorm_meta_maps_entry_points() {
    let php = render(&sample(), Target::PhpstormMeta, EntityMode::Redeclare).contents;
    assert!(php.contains("namespace PHPSTORM_META"));
    assert!(php.contains("override(\\XF::finder(0), map(["));
    assert!(php.contains("'XF:User' => \\XFIDEHelper\\Finder_XF_User::class,"));
    assert!(php.contains("\\XF\\Finder\\User::class => \\XFIDEHelper\\Finder_XF_User::class,"));
    assert!(php.contains("override(\\XF\\Mvc\\Entity\\Manager::find(0), map(["));
    assert!(php.contains("'XF:User' => \\XF\\Entity\\User::class,"));
    assert!(php.contains("override(\\XF::repository(0), map(["));
    assert!(php.contains("\\XF\\Repository\\User::class => \\XF\\Repository\\User::class,"));
    assert!(php.contains("override(\\XF::service(0), map(["));
    assert!(php.contains("'XF:User\\\\Registration' => \\XF\\Service\\User\\Registration::class,"));
    assert!(php.contains("override(\\XF\\Mvc\\Controller::finder(0), map(["));
    assert!(php.contains("override(\\XF\\Service\\AbstractService::repository(0), map(["));
    assert!(php.contains("override(\\XF\\Mvc\\Entity\\Repository::finder(0), map(["));
    assert!(php.contains("override(\\XF\\Mvc\\Controller::assertRecordExists(0), map(["));
    assert!(php.contains("override(\\XF\\App::captcha(0), map(["));
    assert!(php.contains("'XF:Turnstile' => \\XF\\Captcha\\Turnstile::class,"));
}

#[test]
fn xfcp_emits_proxy_stubs() {
    let php = render(&sample(), Target::Xfcp, EntityMode::Redeclare).contents;
    assert!(php.contains("namespace Vendor\\AddOn\\XF\\Repository {"));
    assert!(php.contains("class XFCP_User extends \\XF\\Repository\\User {}"));
}

#[test]
fn options_emit_property_stub() {
    let php = render(&sample(), Target::Options, EntityMode::Redeclare).contents;
    assert!(php.contains("class Options extends \\ArrayObject {}"));
    assert!(php.contains("@property string $boardTitle"));
    assert!(php.contains("@property int $maxAttachmentSize"));
    assert!(php.contains("@method static \\XFIDEHelper\\Options options()"));
    assert!(php.contains("@method \\XFIDEHelper\\Options options()"));
}

#[test]
fn getter_shadowed_column_uses_bypass_suffix() {
    let php = render(&sample(), Target::IdeHelper, EntityMode::Redeclare).contents;
    assert!(php.contains("$last_activity_"));
}
