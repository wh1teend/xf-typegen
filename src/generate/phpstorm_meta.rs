use crate::contract::Contract;
use crate::php::{class_const, finder_stub_ident, single_quoted};
use indexmap::IndexMap;
use std::fmt::Write;

const ENTITY_TARGETS: &[&str] = &[
    "\\XF\\Mvc\\Entity\\Manager::find(0)",
    "\\XF\\Mvc\\Entity\\Manager::create(0)",
    "\\XF\\Mvc\\Controller::assertRecordExists(0)",
    "\\XF\\Mvc\\Controller::assertViewableRecord(0)",
];

const FINDER_TARGETS: &[&str] = &[
    "\\XF::finder(0)",
    "\\XF\\App::finder(0)",
    "\\XF\\Mvc\\Entity\\Manager::getFinder(0)",
    "\\XF\\Mvc\\Controller::finder(0)",
    "\\XF\\Mvc\\Entity\\Repository::finder(0)",
    "\\XF\\Mvc\\Entity\\Entity::finder(0)",
    "\\XF\\Service\\AbstractService::finder(0)",
    "\\XF\\Widget\\AbstractWidget::finder(0)",
    "\\XF\\ActivitySummary\\AbstractSection::finder(0)",
    "\\XF\\AddOn\\DataType\\AbstractDataType::finder(0)",
];

const REPOSITORY_TARGETS: &[&str] = &[
    "\\XF::repository(0)",
    "\\XF\\App::repository(0)",
    "\\XF\\Mvc\\Entity\\Manager::getRepository(0)",
    "\\XF\\Mvc\\Controller::repository(0)",
    "\\XF\\Mvc\\Entity\\Repository::repository(0)",
    "\\XF\\Mvc\\Entity\\Entity::repository(0)",
    "\\XF\\Service\\AbstractService::repository(0)",
    "\\XF\\Widget\\AbstractWidget::repository(0)",
    "\\XF\\ActivitySummary\\AbstractSection::repository(0)",
];

const SERVICE_TARGETS: &[&str] = &[
    "\\XF::service(0)",
    "\\XF\\App::service(0)",
    "\\XF\\Mvc\\Controller::service(0)",
    "\\XF\\Service\\AbstractService::service(0)",
    "\\XF\\Widget\\AbstractWidget::service(0)",
    "\\XF\\ActivitySummary\\AbstractSection::service(0)",
];

const CAPTCHA_TARGETS: &[&str] = &["\\XF\\App::captcha(0)"];

#[derive(Clone, Copy)]
enum MapKind {
    Entity,
    Finder,
    Repository,
}

pub fn render(contract: &Contract) -> String {
    let mut out = super::banner("");
    out.push_str("namespace PHPSTORM_META {\n\n");

    emit_overrides(&mut out, "Entity resolution", ENTITY_TARGETS, &entity_entries(contract, MapKind::Entity));
    emit_overrides(&mut out, "Finder resolution", FINDER_TARGETS, &entity_entries(contract, MapKind::Finder));
    emit_overrides(&mut out, "Repository resolution", REPOSITORY_TARGETS, &entity_entries(contract, MapKind::Repository));
    emit_overrides(&mut out, "Service resolution", SERVICE_TARGETS, &pair_entries(&contract.services));
    emit_overrides(&mut out, "Captcha resolution", CAPTCHA_TARGETS, &pair_entries(&contract.captchas));

    out.push_str("}\n");
    out
}

fn entity_entries(contract: &Contract, kind: MapKind) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for (short, entity) in &contract.entities {
        match kind {
            MapKind::Entity => {
                entries.push((single_quoted(short), class_const(&entity.class)));
            }
            MapKind::Finder => {
                let stub = format!("\\XFIDEHelper\\{}::class", finder_stub_ident(short));
                entries.push((single_quoted(short), stub.clone()));
                if let Some(finder) = &entity.finder {
                    entries.push((class_const(finder), stub));
                }
            }
            MapKind::Repository => {
                if let Some(repo) = &entity.repository {
                    let repo_class = class_const(repo);
                    entries.push((single_quoted(short), repo_class.clone()));
                    entries.push((repo_class.clone(), repo_class));
                }
            }
        }
    }
    entries
}

fn pair_entries(map: &IndexMap<String, String>) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    for (short, class) in map {
        let class_const = class_const(class);
        entries.push((single_quoted(short), class_const.clone()));
        entries.push((class_const.clone(), class_const));
    }
    entries
}

fn emit_overrides(out: &mut String, label: &str, targets: &[&str], entries: &[(String, String)]) {
    if entries.is_empty() {
        return;
    }

    let mut map = String::new();
    for (key, value) in entries {
        let _ = write!(map, "\t\t\t{} => {},\n", key, value);
    }

    let _ = write!(out, "\t// {}\n", label);
    for target in targets {
        let _ = write!(out, "\toverride({}, map([\n{}\t]));\n", target, map);
    }
    out.push('\n');
}
