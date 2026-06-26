# xf-typegen

> Typed IDE stubs for the parts of XenForo the framework doesn't type itself.

## The problem

Straight up: XenForo already types most of its own runtime magic, so this is a narrow tool by
design. Worth knowing what core already does before you reach for it.

On **both 2.2 and 2.3**, XF ships `xf-dev:generate-phpstorm-meta`, which writes a `.phpstorm.meta.php`
that resolves the string-style call sites — `\XF::finder('XF:User')`, `repository()`, `service()`,
`em()->find()`, the `$this->finder(...)` helpers in controllers/repos/services/widgets, and more.
Entities ship full `@property` blocks for their columns, relations and getters. On **2.3**, template
generics additionally type the `::class`-style call sites and the finder chain
(`\XF::finder(\XF\Finder\UserFinder::class)->fetchOne()` → `\XF\Entity\User`). For standard code on
2.3, that's basically everything — you probably don't need this.

What XF doesn't type, on **either** version:

- **Board options.** `\XF::options()->boardTitle` comes back `mixed` — a bare `\ArrayObject` on 2.2,
  and `XF\Options` with `#[AllowDynamicProperties]` but no per-key `@property` on 2.3.
- **XFCP proxies.** `class Foo extends XFCP_Foo` points at a class XF builds at runtime with no
  file, so the IDE flags it undefined. Nothing in core generates a stub for it.
- **Cross-add-on columns.** A column another add-on adds to an entity via XFCP or the
  `entity_structure` event isn't in that entity's shipped `@property` — XF generates that against
  the bare class, not the composed one.

And one gap that's **2.2-only**:

- **The finder chain.** 2.2 has no generics and its finders carry no typed methods, so
  `finder('XF:User')->fetchOne()` lands on the base `Entity`, not the concrete one. 2.3 types this
  natively; 2.2 doesn't.

That's the whole scope. xf-typegen fills exactly those gaps. If you're on 2.3 and none of them bite
you, the stock dev tools already have you covered.

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

Listed by how much they actually add over stock XF — the first two are the reason this exists.

**`_ide_helper_options.php`** — the one core never covers on either version. `\XF::options()` is a
bare `\ArrayObject` on 2.2, and `XF\Options` with `#[AllowDynamicProperties]` but no per-key
`@property` on 2.3, so `\XF::options()->boardTitle` is `mixed` both ways. This declares an
`XFIDEHelper\Options` class with one `@property` per option (type inferred from the current value)
and redeclares `XF::options()` / `App::options()` to return it:

```php
\XF::options()->boardTitle;   // string
```

Caveat: because it redeclares `\XF` / `\XF\App`, Intelephense ends up with two `options()`
declarations (the real one and the typed stub), so hover shows both and you may get a "duplicate
declaration" notice. It's cosmetic — Intelephense merges them, completion works, the rest of `\XF::`
is unaffected. If the doubled hover bothers you, delete this file and add
`@return \XFIDEHelper\Options` to the two `options()` docblocks in XF core instead.

**`_ide_helper_xfcp.php`** — also never covered by core. When an add-on writes
`class Foo extends XFCP_Foo`, XF builds that `XFCP_Foo` proxy at runtime with no file, so the IDE
reports it as undefined. This declares each proxy so the chain resolves:

```php
namespace Vendor\AddOn\XF\Repository {
    class XFCP_ConnectedAccount extends \XF\Repository\ConnectedAccount {}
}
```

**`_ide_helper.php`** — typed Finder/Collection stubs plus entity `@property`. The Finder/Collection
typing is the part that matters **on 2.2**: there `finder('XF:User')->fetchOne()` returns the base
`Entity`, and these stubs make the whole chain
(`finder('XF:User')->where(...)->fetchOne()`, `->fetch()`, `first()`, `filter()`, …) resolve to the
concrete entity. On 2.3 generics already do this, so it's redundant there. The entity `@property`
part is also mostly redundant — XF ships those blocks — except it's built from the *composed*
structure, so it includes columns other add-ons add via XFCP. Two strategies (`--entity-mode`):

- **`redeclare`** (default) redeclares each entity class with its properties, like Laravel's
  `_ide_helper_models`. No source edits; some analyzers warn about the duplicate class.
- **`mixin`** emits `XFIDEHelper\Entity_<id>` classes attached via a `@mixin` line that
  `xf-typegen extract --mixin apply` writes into your entity files. No duplicate-class warnings, but
  it edits the XF tree (idempotent; `--mixin remove` reverses it). Use with
  `xf-typegen generate --entity-mode mixin`.

**`.phpstorm.meta.php`** — resolves the string-style call sites
(`\XF::finder('XF:User')`, `repository()`, `service()`, `em()->find()`, `$this->finder(...)`, …).
Note this **duplicates XF's own `xf-dev:generate-phpstorm-meta`**, which ships in both 2.2 and 2.3 —
it's here so you get everything from one command, but if you already run the stock generator, skip
this target with `--targets options,xfcp,ide-helper` (the four targets are `ide-helper`,
`phpstorm-meta`, `xfcp`, `options`).
