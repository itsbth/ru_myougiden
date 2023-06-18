#![deny(clippy::all)]
#![deny(clippy::pedantic)]
use crate::indexer::create_index;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use itertools::izip;
use itertools::Itertools;
use std::path::Path;
use tantivy::schema::Schema;
use tantivy::{DocAddress, Document, Index, Score, Searcher};
use yansi::{Color, Paint, Style};

mod indexer;

#[derive(clap::ValueEnum, Clone)]
enum Field {
    Word,
    Reading,
    ReadingRomaji,
    Meaning,
}

#[derive(clap::ValueEnum, Clone, Debug)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[clap(short, long, global = true, env = "AKASABI_INDEX")]
    index: Option<String>,
    #[clap(long, global = true, default_value = "auto")]
    color: ColorArg,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Search {
        term: String,
        #[clap(short, long)]
        field: Option<Field>,
        #[clap(short, long)]
        create_if_missing: bool,
    },
    Index {
        #[clap(short, long, help = "Path to JMdict.gz file")]
        path: String,
        #[cfg(feature = "http")]
        #[clap(
            short,
            long,
            help = "Automatically download the latest JMdict.gz file if it doesn't exist"
        )]
        jmdict_url: Option<String>,
    },
}

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    {
        fn detect_color() -> bool {
            // not tty -> false
            // NO_COLOR -> false
            // else -> true
            #[cfg(unix)]
            {
                if !nix::unistd::isatty(nix::libc::STDOUT_FILENO).unwrap_or(false) {
                    return false;
                }
            }
            if let Ok(no_color) = std::env::var("NO_COLOR") {
                return no_color.is_empty();
            }
            true
        }
        let color = match args.color {
            ColorArg::Auto => detect_color(),
            ColorArg::Always => true,
            ColorArg::Never => false,
        };

        if color {
            Paint::enable_windows_ascii();
            Paint::enable();
        } else {
            Paint::disable();
        }
    }

    let index_path = args.index.unwrap_or("./tmp".to_string());

    let schema = indexer::create_schema();

    let index = if Path::new(&index_path).join("meta.json").exists() {
        Index::open_in_dir(&index_path).context("Failed to open index")?
    } else {
        Index::create_in_dir(&index_path, schema.clone()).context("Failed to create index")?
    };

    match args.command {
        Command::Search {
            term,
            field,
            create_if_missing: _,
        } => {
            let (searcher, top_docs) = search(&index, &schema, &term, &field)?;

            for (_score, doc_address) in top_docs {
                let retrieved_doc = searcher.doc(doc_address)?;
                print_result(&schema, &retrieved_doc);
            }
        }
        Command::Index { path, .. } => {
            index_(&index, &schema, &path)?;
        }
    }

    Ok(())
}

fn index_(index: &Index, schema: &Schema, path: &str) -> Result<()> {
    create_index(schema, path, index)?;
    Ok(())
}

fn search(
    index: &Index,
    schema: &Schema,
    term: &str,
    field: &Option<Field>,
) -> Result<(Searcher, Vec<(Score, DocAddress)>)> {
    let (word, reading, reading_romaji, meaning) = (
        schema.get_field("word").unwrap(),
        schema.get_field("reading").unwrap(),
        schema.get_field("reading_romaji").unwrap(),
        schema.get_field("meaning").unwrap(),
    );

    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::Manual)
        .try_into()?;
    let searcher = reader.searcher();

    let fields = match field {
        Some(Field::Word) => vec![word],
        Some(Field::Reading) => vec![reading],
        Some(Field::ReadingRomaji) => vec![reading_romaji],
        Some(Field::Meaning) => vec![meaning],
        None => vec![word, reading, reading_romaji, meaning],
    };

    let query_parser = tantivy::query::QueryParser::for_index(index, fields);

    let query = query_parser.parse_query(term)?;

    let top_docs = searcher.search(&query, &tantivy::collector::TopDocs::with_limit(10))?;

    Ok((searcher, top_docs))
}

// TODO: Also take query so we can highlight it
fn print_result(schema: &Schema, document: &Document) {
    // entry fields
    let word = schema.get_field("word").unwrap();
    let reading = schema.get_field("reading").unwrap();
    // let reading_romaji = schema.get_field("reading_romaji").unwrap();

    // sense fields
    let meaning = schema.get_field("meaning").unwrap();
    let pos = schema.get_field("pos").unwrap();
    let field = schema.get_field("field").unwrap();

    // myougiden format:
    // kanji [;kanji]* (reading [、reading]*)*
    // 1. \[poc\] meaning [; meaning]*
    // 2. \[field\] meaning [; meaning]*

    let kanji = document
        .get_all(word)
        .map(|f| f.as_text().unwrap())
        .collect_vec();
    let readings = document
        .get_all(reading)
        .map(|f| f.as_text().unwrap())
        .collect_vec();

    // meanings, pos, and fields should be "aligned" (ie. same length, n-th element of each)
    let meanings = document
        .get_all(meaning)
        .map(|f| f.as_text().unwrap())
        .collect_vec();
    let pos = document
        .get_all(pos)
        .map(|f| f.as_text().unwrap())
        .collect_vec();
    let fields = document
        .get_all(field)
        .map(|f| f.as_text().unwrap())
        .collect_vec();

    let c_kanji = Style::new(Color::Blue).bold();
    let c_reading = Style::new(Color::Magenta).bold();

    // field shares style with pos
    let c_pos = Style::new(Color::Yellow).bold();
    let c_meaning = Style::new(Color::Default).bold();
    let c_index = Style::new(Color::Green).bold();

    if kanji.is_empty() {
        println!("{}", c_kanji.paint(readings.join("、")));
    } else {
        println!(
            "{} ({})",
            // TODO: Style separator separately
            c_kanji.paint(kanji.join("; ")),
            c_reading.paint(readings.join("、"))
        );
    }

    for (idx, (meaning, pos, field)) in izip!(meanings, pos, fields).enumerate() {
        let meanings = meaning.split("; ").collect_vec();

        // TODO: Properly handle pos and field (split and re-join)
        print!(
            "{} [{};{}]",
            c_index.paint(format!("{}.", idx + 1)),
            c_pos.paint(pos),
            c_pos.paint(field)
        );
        for (idx, meaning) in meanings.iter().enumerate() {
            if idx == 0 {
                print!(" {}", c_meaning.paint(meaning));
                continue;
            }
            print!("{}{}", Paint::yellow("; "), c_meaning.paint(meaning));
        }
        println!();
    }
    println!();
}
