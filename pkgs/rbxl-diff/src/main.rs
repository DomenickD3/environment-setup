use std::{
    collections::HashMap,
    env,
    fs::File,
    io::{BufReader, Write},
    path::Path,
};

use anyhow::{Context, Result};
use rbx_binary::Deserializer;
use rbx_reflection::DataType;
use rbx_dom_weak::types::VariantType;

fn main() -> Result<()> {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .context("usage: rbxl-diff <place.rbxl> [output.txt]")?;
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

fn dump_place(path: &Path) -> Result<Vec<String>> {
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
        .with_context(|| {
        format!("failed to parse {}", path.display())
    })?;

    let mut lines = Vec::new();
    walk(&dom, dom.root_ref(), "DataModel".to_string(), &mut lines);
    Ok(lines)
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
