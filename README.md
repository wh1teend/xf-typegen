# xf-typegen

> Typed IDE stubs for the parts of XenForo the framework doesn't type itself.

## The problem

What you get out of this depends on your XF version, so let's be straight about it.

**On XF 2.3 you may not need it.** The framework already types most of this itself: entities ship
full `@property` blocks, finders are generic (`\XF::finder(\XF\Finder\User::class)->fetchOne()`
resolves to `\XF\Entity\User`), and the magic call sites resolve through `class-string<T>` when you
use the `::class` style. If that covers your code, check before bothering with this.

**On XF 2.2 none of that exists.** `finder()` / `repository()` are typed only to the base
`\XF\Mvc\Entity\Finder` / `Repository`, `em()->find()` to `Entity`, finders carry no typed methods,
and `\XF::options()` is a bare `\ArrayObject`:

```php
// XF 2.2 — what the IDE actually sees
\XF::finder('XF:User')->fetchOne();   // \XF\Mvc\Entity\Finder, then mixed
\XF::repository('XF:User');           // \XF\Mvc\Entity\Repository (base)
\XF::em()->find('XF:User', 1);        // \XF\Mvc\Entity\Entity (base)
\XF::options()->boardTitle;           // mixed
```

xf-typegen resolves every one of those to the concrete class — so on 2.2 you get the same
concrete-type resolution that 2.3 gets for free from generics.

**On both versions** it also covers a few things core never types:

- board options per key — `\XF::options()->boardTitle` as `string`, not `mixed` (2.3's `XF\Options`
  uses `#[AllowDynamicProperties]` but declares no per-key `@property`);
- the runtime `XFCP_*` class-extension proxies, which have no file for the IDE to read;
- columns and relations another add-on adds to an entity via XFCP or the `entity_structure` event —
  those aren't in that entity's shipped `@property`, because XF generates it against the bare class.

One thing it is *not* for: plain entity property typing (`$user->username`). XF ships those
`@property` blocks itself on both 2.2 and 2.3 — the only thing this adds there is the
XFCP-composed columns above.

## Compared to `xf-dev:entity-class-properties`

XF's built-in command writes `@property` lines into your entity files — the same data the core
entities already ship with. It doesn't touch the call sites, the finder chain, options or XFCP at
all, so the two barely overlap. The only shared ground is entity `@property`, and even there
`xf-typegen` differs:

| | `xf-dev:entity-class-properties` | `xf-typegen` |
|---|---|---|
| Entity `@property` | yes | yes |
| Includes columns/relations *other* add-ons add via XFCP | no | yes |
| Magic call sites (`finder`, `repository`, `em()->find`) | — | resolved |
| `fetchOne()` / `fetch()` / iteration typed | — | yes |
| Board options / XFCP proxies | — | yes |
| Touches your source files | always | only if you opt in |

On the `@property` side the difference is **composition**: XF's command calls `getStructure()` on
the bare base class, so anything another add-on adds through an XFCP extension (or the
`entity_structure` event) is invisible to it. `xf-typegen` requests the *composed* structure
(`Manager::getEntityStructure()`), so those add-on columns and relations are included — and it does
it without editing your source unless you opt in (`--entity-mode mixin`).

Everything else `xf-typegen` does — call sites, finder/collection typing, options, XFCP proxies —
the dev command doesn't attempt.

## How it works

Two halves, split on purpose:

```
xf-typegen extract   ── runs the embedded extract.php under PHP, inside XF
      │   reads the runtime-composed structures
      ▼
xf-typegen.json   (the contract — plain JSON)
      │
      ▼
xf-typegen generate   (Rust — runs anywhere)
      ├─► _ide_helper.php           @property stubs + typed Finder/Collection
      ├─► .phpstorm.meta.php        string & ::class → entity/finder/repo
      ├─► _ide_helper_xfcp.php      XFCP_* class-extension proxies
      └─► _ide_helper_options.php   typed \XF::options()
```

Only PHP, booted inside XF, can see what an entity really looks like after the full extension
chain is composed, so that half has to be PHP. Everything after that is string assembly, which
Rust handles (fast, with a watch mode and incremental writes). The JSON contract in the middle
keeps the two halves independent.

Both halves ship as one binary. `extract.php` is embedded into it, so `xf-typegen extract` runs
the script through PHP and `xf-typegen generate` writes the stubs. Nothing to keep in sync by
hand, and the script can't drift away from the binary. It does still need a `php` available to
boot XF.

## Setup

### 1. Build it

```sh
cargo build --release
# binary lands at: target/release/xf-typegen
```

### 2. Extract the contract

The extractor needs a PHP that can boot your XF install. Any local stack works (Open Server,
Laragon, MAMP, XAMPP, or plain system PHP). The binary carries `extract.php` inside it, so point
the `extract` subcommand at your XF root:

```sh
xf-typegen extract /path/to/xenforo --out /path/to/xenforo/xf-typegen.json
```

Worth knowing:

- The **database** credentials come from the install's own `config.php`, so a local MySQL on
  `localhost` works with no extra setup.
- **PHP version**: use one your XF supports (2.2 → 7.x/8.0–8.1, 2.3 → 8.x). If your PHP is newer
  than XF officially supports it still runs; deprecation notices go to stderr so the JSON stays
  clean.
- The contract always comes back over stdout and the binary writes the file itself, so `--out`
  lands wherever you point it.
- `--php-cmd` selects the interpreter if it isn't plain `php` on your `PATH`, e.g.
  `--php-cmd php8.1`.
- `--addon=Vendor/AddOn` limits extraction to a single add-on; `--minify` produces compact JSON.

<details>
<summary>Or run <code>extract.php</code> directly</summary>

If you'd rather skip the binary, the raw script still works on its own:

```sh
php extract.php /path/to/xenforo --out=/path/to/xenforo/xf-typegen.json
```
</details>

### 3. Generate the stubs

```sh
target/release/xf-typegen generate -i /path/to/xenforo/xf-typegen.json
```

By default everything lands next to the contract (i.e. in your XF root). Common flags:

```sh
# preview without writing anything
... generate -i xf-typegen.json --dry-run

# a single target
... generate -i xf-typegen.json --targets phpstorm-meta

# rebuild automatically whenever the contract changes
... watch -i /path/to/xenforo/xf-typegen.json
```

### 4. Point your editor at it

The generated files (`_ide_helper.php`, `.phpstorm.meta.php`, `_ide_helper_xfcp.php`,
`_ide_helper_options.php`) sit in the project and get picked up automatically. One rule: **keep
the `_ide_helper*.php` files out of your PHP autoload.** They exist for static analysis only and
should never run.

That's the loop. Change an entity, re-run steps 2 and 3 (a small wrapper script around both is the
usual approach). Adding a column or a new entity needs no XF rebuild; changing an XFCP class
extension does, so run `xf-dev:rebuild-caches` after that.

## What gets generated

**`.phpstorm.meta.php`** resolves the magic call sites to concrete classes, in both styles XF
accepts:

```php
\XF::finder('XF:User')                 // string style
\XF::finder(\XF\Finder\User::class)    // ::class style (XF 2.3, also fine in 2.2)
\XF::repository('XF:User')
\XF::service('XF:User\Registration')   // services too, both styles
\XF::app()->captcha('XF:Turnstile')    // captchas
\XF::em()->find('XF:User', 1)
$this->finder('XF:User');              // same helpers inside controllers,
$this->repository('XF:User');          //   repositories, services, widgets, …
$this->assertRecordExists('XF:User', $id);
```

**`_ide_helper.php`** holds the bulk of it:

- typed **Finder** stubs, so `finder('XF:User')->fetchOne()` returns the entity, and the fluent
  chain keeps the type the whole way down:
  `finder('XF:User')->where(...)->order(...)->fetchOne()` still resolves to the entity.
- typed **Collection** stubs, so `->fetch()` returns an entity-typed collection. `foreach`,
  `first()`, `last()`, `toArray()`, `filter()/slice()/reverse()` all resolve to the entity. This
  is done per-entity, so XF's real `Finder`/`AbstractCollection` methods stay completable and
  nothing core is redeclared.
- plain entity **`@property`** stubs for columns, getters and relations.

The entity `@property` part has two strategies (`--entity-mode`):

- **`redeclare`** (the default) redeclares each entity class with its properties, like Laravel's
  `_ide_helper_models`. It doesn't touch your source, but a few analyzers will warn about the
  duplicate class.
- **`mixin`** emits `XFIDEHelper\Entity_<id>` helper classes and attaches them with a `@mixin`
  line that `xf-typegen extract --mixin apply` writes into your entity files. No duplicate-class
  warnings, but it edits the XF tree. It's idempotent, and `--mixin remove` reverses it exactly.
  Use both sides together: `xf-typegen extract --mixin apply`, then
  `xf-typegen generate --entity-mode mixin`.

Stick with `redeclare` unless your editor complains.

**`_ide_helper_xfcp.php`** covers XF's class-extension proxies. When an add-on extends a class it
writes `class Foo extends XFCP_Foo`, and XF builds that `XFCP_Foo` proxy at runtime. There's no
file for it anywhere, so the IDE reports it as undefined. This file declares each proxy so the
chain resolves:

```php
namespace Vendor\AddOn\XF\Repository {
    class XFCP_ConnectedAccount extends \XF\Repository\ConnectedAccount {}
}
```

**`_ide_helper_options.php`** types board options, which core leaves untyped on both versions:
`\XF::options()` is a bare `\ArrayObject` on 2.2, and on 2.3 it's `XF\Options` with
`#[AllowDynamicProperties]` but no per-key `@property` — so `\XF::options()->boardTitle` comes back
`mixed` either way. This declares an `XFIDEHelper\Options` class with one `@property` per option
(the type inferred from the current value) and redeclares `XF::options()` / `App::options()` to
return it, so the option resolves with no `@var` hints and no edits to XF core:

```php
\XF::options()->boardTitle;   // string
```

One caveat: because it redeclares `\XF` / `\XF\App`, Intelephense ends up with two `options()`
declarations (the real one and the typed stub), so hover shows both and you may get a "duplicate
declaration" notice. It's cosmetic; Intelephense merges the declarations, so completion still works
and the rest of `\XF::` is unaffected. If the doubled hover bothers you, delete this file and add
`@return \XFIDEHelper\Options` to the two `options()` docblocks in XF core instead.
