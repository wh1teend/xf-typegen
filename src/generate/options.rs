use crate::contract::Contract;
use std::fmt::Write;

pub fn render(contract: &Contract) -> String {
    let mut out = super::banner(
        " *\n * @property stubs for board options, plus a re-typing of XF::options() /\n * App::options() so \\XF::options()->name resolves with no edits to XF core.\n * If this ever hides \\XF:: members, delete this single file.\n",
    );

    out.push_str("namespace XFIDEHelper {\n\t/**\n");
    for (name, ty) in &contract.options {
        let _ = write!(out, "\t * @property {} ${}\n", ty, name);
    }
    out.push_str("\t */\n\tclass Options extends \\ArrayObject {}\n}\n\n");

    out.push_str(
        "namespace {\n\t/**\n\t * @method static \\XFIDEHelper\\Options options()\n\t */\n\tclass XF {}\n}\n\n",
    );
    out.push_str(
        "namespace XF {\n\t/**\n\t * @method \\XFIDEHelper\\Options options()\n\t */\n\tclass App {}\n}\n",
    );

    out
}
