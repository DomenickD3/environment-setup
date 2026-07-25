use std::{
    collections::{HashMap, HashSet},
    env,
    fs::{self, File, OpenOptions},
    io::{BufReader, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use rbx_binary::Deserializer;
use rbx_dom_weak::{types::Variant, ustr, Instance, WeakDom};
use rbx_reflection::DataType;
use rbx_dom_weak::types::{Ref, VariantType};

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .context(
            "usage: rbxl-diff <place.rbxl> [output.txt]\n       rbxl-diff export --input <place.rbxl> --out <project-dir> [--clean] [--write-project-json]",
        )?;

    if input == "export" {
        return export_command(args.collect());
    }

    let output = args.next();

    let mut lines = dump_place(Path::new(&input))?;
    lines.sort();

    match output {
        Some(path) => {
            let mut file = File::create(path)?;
            for line in lines {
                writeln!(file, "{line}")?;
            }
        }
        None => {
            for line in lines {
                println!("{line}");
            }
        }
    }

    Ok(())
}

fn export_command(args: Vec<String>) -> Result<()> {
    let mut input = None;
    let mut out = None;
    let mut clean = false;
    let mut write_project_json = false;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--input" => {
                index += 1;
                input = Some(PathBuf::from(
                    args.get(index).context("--input requires a path")?,
                ));
            }
            "--out" => {
                index += 1;
                out = Some(PathBuf::from(args.get(index).context("--out requires a path")?));
            }
            "--clean" => clean = true,
            "--write-project-json" => write_project_json = true,
            other => bail!("unknown export argument: {other}"),
        }
        index += 1;
    }

    let input = input.context("export requires --input <place.rbxl>")?;
    let out = out.context("export requires --out <project-dir>")?;
    let dom = parse_place(&input)?;

    if clean {
        for relative in [
            "src/shared",
            "src/server",
            "src/client",
            "src/remotes",
            "src/items",
        ] {
            let path = out.join(relative);
            if path.exists() {
                fs::remove_dir_all(&path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
        }
    }

    let mut exporter = Exporter::new(out.clone());
    exporter.export_mapping(
        &dom,
        "ReplicatedStorage/ModuleScripts",
        Path::new("src/shared"),
    )?;
    exporter.export_mapping(&dom, "ServerScriptService", Path::new("src/server"))?;
    exporter.export_mapping(
        &dom,
        "StarterPlayer/StarterPlayerScripts",
        Path::new("src/client"),
    )?;
    exporter.export_mapping(
        &dom,
        "ReplicatedStorage/RemoteEvents",
        Path::new("src/remotes"),
    )?;
    exporter.export_mapping(&dom, "ReplicatedStorage/Items", Path::new("src/items"))?;

    if write_project_json {
        write_project_json_file(&out, exporter.stats.items_exported > 0)?;
    }

    exporter.print_summary();
    Ok(())
}

fn dump_place(path: &Path) -> Result<Vec<String>> {
    let dom = parse_place(path)?;
    let mut lines = Vec::new();
    walk(&dom, dom.root_ref(), "DataModel".to_string(), &mut lines);
    Ok(lines)
}

fn parse_place(path: &Path) -> Result<WeakDom> {
    let input = BufReader::new(File::open(path).with_context(|| {
        format!("failed to open {}", path.display())
    })?);
    let mut database = rbx_reflection_database::get()
        .context("failed to load Roblox reflection database")?
        .clone();
    patch_tags_properties(&mut database);

    let dom = Deserializer::new()
        .reflection_database(&database)
        .deserialize(input)
        .with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(dom)
}

fn patch_tags_properties(database: &mut rbx_reflection::ReflectionDatabase<'static>) {
    for class in database.classes.values_mut() {
        if let Some(property) = class.properties.get_mut("Tags") {
            if matches!(property.data_type, DataType::Value(VariantType::Tags)) {
                property.data_type = DataType::Value(VariantType::SharedString);
            }
        }
    }
}

fn walk(
    dom: &rbx_dom_weak::WeakDom,
    referent: rbx_dom_weak::types::Ref,
    path: String,
    lines: &mut Vec<String>,
) {
    let Some(instance) = dom.get_by_ref(referent) else {
        return;
    };

    lines.push(format!(
        "INSTANCE\t{}\t{}\t{}",
        path,
        instance.class,
        instance.name
    ));

    let mut properties: Vec<_> = instance.properties.iter().collect();
    properties.sort_by_key(|(name, _)| name.to_string());
    for (name, value) in properties {
        lines.push(format!("PROPERTY\t{}\t{}\t{:?}", path, name, value));
    }

    let mut children: Vec<_> = instance
        .children()
        .iter()
        .filter_map(|child_ref| dom.get_by_ref(*child_ref).map(|child| (*child_ref, child)))
        .collect();
    children.sort_by(|(_, left), (_, right)| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.class.to_string().cmp(&right.class.to_string()))
            .then_with(|| format!("{:?}", left.referent()).cmp(&format!("{:?}", right.referent())))
    });

    let mut duplicate_counts: HashMap<(String, String), usize> = HashMap::new();
    for (_, child) in &children {
        *duplicate_counts
            .entry((child.name.clone(), child.class.to_string()))
            .or_default() += 1;
    }

    let mut seen_counts: HashMap<(String, String), usize> = HashMap::new();
    for (child_ref, child) in children.into_iter() {
        let key = (child.name.clone(), child.class.to_string());
        let occurrence = seen_counts.entry(key.clone()).or_default();
        let suffix = if duplicate_counts.get(&key).copied().unwrap_or(0) > 1 {
            format!("#{}", *occurrence)
        } else {
            String::new()
        };
        *occurrence += 1;

        let child_path = format!(
            "{}/{}[{}{}]",
            path,
            escape_path_segment(&child.name),
            child.class,
            suffix
        );
        walk(dom, child_ref, child_path, lines);
    }
}

fn escape_path_segment(value: &str) -> String {
    value.replace('\\', "\\\\").replace('/', "\\/")
}

#[derive(Default)]
struct ExportStats {
    module_scripts: usize,
    scripts: usize,
    local_scripts: usize,
    remotes: usize,
    items_exported: usize,
    skipped: usize,
}

struct Exporter {
    out: PathBuf,
    written_paths: HashSet<PathBuf>,
    stats: ExportStats,
    warnings: Vec<String>,
}

impl Exporter {
    fn new(out: PathBuf) -> Self {
        Self {
            out,
            written_paths: HashSet::new(),
            stats: ExportStats::default(),
            warnings: Vec::new(),
        }
    }

    fn export_mapping(
        &mut self,
        dom: &WeakDom,
        source_path: &str,
        output_path: &Path,
    ) -> Result<()> {
        let Some(root_ref) = find_instance_path(dom, source_path) else {
            self.warn(format!("source path not found: {source_path}"));
            return Ok(());
        };

        let output_root = self.out.join(output_path);
        fs::create_dir_all(&output_root)
            .with_context(|| format!("failed to create {}", output_root.display()))?;

        let root = dom
            .get_by_ref(root_ref)
            .with_context(|| format!("source path disappeared: {source_path}"))?;
        for child_ref in sorted_children(dom, root) {
            self.export_instance(dom, child_ref, &output_root, source_path, output_path)?;
        }

        Ok(())
    }

    fn export_instance(
        &mut self,
        dom: &WeakDom,
        referent: Ref,
        directory: &Path,
        roblox_parent_path: &str,
        relative_output_root: &Path,
    ) -> Result<()> {
        let Some(instance) = dom.get_by_ref(referent) else {
            return Ok(());
        };

        let roblox_path = format!("{}/{}", roblox_parent_path, instance.name);
        match instance.class.as_str() {
            "Folder" => {
                let child_dir = directory.join(sanitize_path_segment(&instance.name));
                fs::create_dir_all(&child_dir)
                    .with_context(|| format!("failed to create {}", child_dir.display()))?;
                for child_ref in sorted_children(dom, instance) {
                    self.export_instance(
                        dom,
                        child_ref,
                        &child_dir,
                        &roblox_path,
                        relative_output_root,
                    )?;
                }
            }
            "ModuleScript" => {
                let path =
                    directory.join(format!("{}.luau", sanitize_path_segment(&instance.name)));
                self.write_source_file(instance, &path, &roblox_path)?;
                self.stats.module_scripts += 1;
                if relative_output_root == Path::new("src/items") {
                    self.stats.items_exported += 1;
                }
            }
            "Script" => {
                let path = directory.join(format!(
                    "{}.server.luau",
                    sanitize_path_segment(&instance.name)
                ));
                self.write_source_file(instance, &path, &roblox_path)?;
                self.stats.scripts += 1;
                if relative_output_root == Path::new("src/items") {
                    self.stats.items_exported += 1;
                }
            }
            "LocalScript" => {
                let path = directory.join(format!(
                    "{}.client.luau",
                    sanitize_path_segment(&instance.name)
                ));
                self.write_source_file(instance, &path, &roblox_path)?;
                self.stats.local_scripts += 1;
                if relative_output_root == Path::new("src/items") {
                    self.stats.items_exported += 1;
                }
            }
            "RemoteEvent" | "RemoteFunction" => {
                let path = directory.join(format!(
                    "{}.model.json",
                    sanitize_path_segment(&instance.name)
                ));
                self.write_text_file(
                    &path,
                    &format!("{{\n  \"ClassName\": \"{}\"\n}}\n", instance.class),
                )?;
                self.stats.remotes += 1;
                if relative_output_root == Path::new("src/items") {
                    self.stats.items_exported += 1;
                }
            }
            _ => {
                self.stats.skipped += 1;
                self.warn(format!(
                    "skipping unsupported instance {roblox_path} ({})",
                    instance.class
                ));
            }
        }

        Ok(())
    }

    fn write_source_file(
        &mut self,
        instance: &Instance,
        path: &Path,
        roblox_path: &str,
    ) -> Result<()> {
        let source = match instance.properties.get(&ustr("Source")) {
            Some(Variant::String(value)) => value.clone(),
            Some(Variant::SharedString(value)) => match String::from_utf8(value.data().to_vec()) {
                Ok(value) => value,
                Err(error) => {
                    self.warn(format!(
                        "Source for {roblox_path} is SharedString but not UTF-8: {error}; writing empty file"
                    ));
                    String::new()
                }
            },
            Some(other) => {
                self.warn(format!(
                    "Source for {roblox_path} is {:?}, not a string; writing empty file",
                    other.ty()
                ));
                String::new()
            }
            None => {
                self.warn(format!("Source missing for {roblox_path}; writing empty file"));
                String::new()
            }
        };

        self.write_text_file(path, &source)
    }

    fn write_text_file(&mut self, path: &Path, contents: &str) -> Result<()> {
        if !self.written_paths.insert(path.to_path_buf()) {
            bail!("duplicate generated output path: {}", path.display());
        }

        let parent = path
            .parent()
            .with_context(|| format!("output path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;

        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .with_context(|| format!("failed to create {}", path.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    fn warn(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    fn print_summary(&self) {
        println!("Export summary:");
        println!("  ModuleScripts: {}", self.stats.module_scripts);
        println!("  Scripts: {}", self.stats.scripts);
        println!("  LocalScripts: {}", self.stats.local_scripts);
        println!("  Remotes: {}", self.stats.remotes);
        println!("  Unsupported skipped: {}", self.stats.skipped);

        if !self.warnings.is_empty() {
            println!("Warnings:");
            for warning in &self.warnings {
                println!("  - {warning}");
            }
        }
    }
}

fn find_instance_path(dom: &WeakDom, path: &str) -> Option<Ref> {
    let mut current_ref = dom.root_ref();
    for segment in path.split('/') {
        let current = dom.get_by_ref(current_ref)?;
        current_ref = current
            .children()
            .iter()
            .copied()
            .find(|child_ref| {
                dom.get_by_ref(*child_ref)
                    .is_some_and(|child| child.name == segment)
            })?;
    }
    Some(current_ref)
}

fn sorted_children(dom: &WeakDom, instance: &Instance) -> Vec<Ref> {
    let mut children: Vec<_> = instance
        .children()
        .iter()
        .copied()
        .filter(|child_ref| dom.get_by_ref(*child_ref).is_some())
        .collect();
    children.sort_by(|left_ref, right_ref| {
        let left = dom.get_by_ref(*left_ref).unwrap();
        let right = dom.get_by_ref(*right_ref).unwrap();
        left.name
            .cmp(&right.name)
            .then_with(|| left.class.to_string().cmp(&right.class.to_string()))
            .then_with(|| format!("{:?}", left.referent()).cmp(&format!("{:?}", right.referent())))
    });
    children
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value.replace(['/', '\0'], "_");
    if sanitized.is_empty() {
        "_".to_string()
    } else {
        sanitized
    }
}

fn write_project_json_file(out: &Path, include_items: bool) -> Result<()> {
    let items_mapping = if include_items {
        r#",

      "Items": {
        "$path": "src/items"
      }"#
    } else {
        ""
    };
    let contents = format!(
        r#"{{
  "name": "zerg-gaming-first-experience",
  "tree": {{
    "$className": "DataModel",

    "ReplicatedStorage": {{
      "$className": "ReplicatedStorage",

      "ModuleScripts": {{
        "$className": "Folder",

        "Abilities": {{
          "$path": "src/shared/Abilities"
        }},

        "Combat": {{
          "$path": "src/shared/Combat"
        }},

        "Inventory": {{
          "$path": "src/shared/Inventory"
        }},

        "UI": {{
          "$path": "src/shared/UI"
        }},

        "Weapons": {{
          "$path": "src/shared/Weapons"
        }}
      }},

      "RemoteEvents": {{
        "$path": "src/remotes"
      }}{items_mapping}
    }},

    "ServerScriptService": {{
      "$className": "ServerScriptService",
      "$path": "src/server"
    }},

    "StarterPlayer": {{
      "$className": "StarterPlayer",

      "StarterPlayerScripts": {{
        "$className": "StarterPlayerScripts",
        "$path": "src/client"
      }}
    }}
  }}
}}
"#
    );

    let path = out.join("default.project.json");
    fs::write(&path, contents).with_context(|| format!("failed to write {}", path.display()))
}
