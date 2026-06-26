#![allow(dead_code)]

use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Contract {
    pub version: u32,
    #[serde(default)]
    pub generator: Option<String>,
    #[serde(default, rename = "generatedAt")]
    pub generated_at: Option<String>,
    pub xf: XfInfo,
    #[serde(default, rename = "activeAddOns")]
    pub active_add_ons: Vec<String>,
    #[serde(default, rename = "classExtensions")]
    pub class_extensions: Vec<ClassExtension>,
    #[serde(default)]
    pub services: IndexMap<String, String>,
    #[serde(default)]
    pub captchas: IndexMap<String, String>,
    #[serde(default)]
    pub options: IndexMap<String, String>,
    pub entities: IndexMap<String, Entity>,
}

#[derive(Debug, Deserialize)]
pub struct ClassExtension {
    pub proxy: String,
    pub extends: String,
}

#[derive(Debug, Deserialize)]
pub struct XfInfo {
    #[serde(rename = "versionId")]
    pub version_id: i64,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Entity {
    #[serde(rename = "shortName")]
    pub short_name: String,
    pub class: String,
    #[serde(default)]
    pub table: Option<String>,
    #[serde(default, rename = "contentType")]
    pub content_type: Option<String>,
    #[serde(default, rename = "primaryKey")]
    pub primary_key: Vec<String>,
    #[serde(default)]
    pub finder: Option<String>,
    #[serde(default)]
    pub repository: Option<String>,
    pub columns: IndexMap<String, Column>,
    pub relations: IndexMap<String, Relation>,
    pub getters: IndexMap<String, Getter>,
}

#[derive(Debug, Deserialize)]
pub struct Column {
    #[serde(rename = "phpType")]
    pub php_type: String,
    pub nullable: bool,
    #[serde(rename = "xfType")]
    pub xf_type: String,
    #[serde(default)]
    pub primary: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cardinality {
    One,
    Many,
}

#[derive(Debug, Deserialize)]
pub struct Relation {
    pub to: Cardinality,
    pub entity: String,
    #[serde(default)]
    pub class: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Getter {
    #[serde(rename = "phpType")]
    pub php_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub source: String,
}

impl Contract {
    pub fn from_path(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading contract: {}", path.display()))?;
        Self::from_str(&raw)
            .with_context(|| format!("parsing contract: {}", path.display()))
    }

    pub fn from_str(raw: &str) -> Result<Self> {
        let contract: Contract = serde_json::from_str(raw)?;
        if contract.version != 1 {
            anyhow::bail!(
                "unsupported contract version {} (this build understands version 1)",
                contract.version
            );
        }
        Ok(contract)
    }
}
