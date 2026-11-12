//! Benchmarks of djs-formatters. Run with `cargo bench -p djs-formatters`.
//!
//! The global `AllocProfiler` reports allocations per operation next to timings; the escaping
//! benchmarks also report throughput in bytes.

use divan::{Bencher, black_box, counter::BytesCount};
use djs_formatters::{
    EscapeOptions, TimestampStyle, bold, code_block, escape_markdown, hyperlink, ordered_list,
    slash_command_mention, time, user_mention,
};
use djs_model::id::{CommandId, UserId};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

/// `text` repeated to `size` bytes (cut at a char boundary).
fn sized(text: &str, size: usize) -> String {
    let mut out = text.repeat(size / text.len() + 1);
    let mut end = size;
    while !out.is_char_boundary(end) {
        end -= 1;
    }
    out.truncate(end);
    out
}

/// Chat text without markdown: what most messages are.
const PLAIN: &str = "Hello there, this is a perfectly ordinary message about the weather. ";
/// Text full of markdown characters.
const HEAVY: &str = "**bold** _it_ __u__ ~~s~~ ||sp|| `c` ```b``` \\ snake_case ***x*** ";
/// Text with links, mentions and emoji, which are copied as they are.
const LINKS: &str =
    "see https://example.com/a_b_c?x=*y* by <@80351110224678912> <:pepe_hi:1> <t:1:R> ";
/// Polish text with a little markdown.
const POLISH: &str = "Zażółć gęślą jaźń — **ważne** ogłoszenie dla _wszystkich_ członków. ";
/// Lines for the line-start options.
const LINES: &str = "# Title\n- item with [a link](https://x.y)\n1. step\nplain line\n";

const SIZES: [usize; 3] = [64, 2048, 65_536];

mod escape {
    use super::{
        BytesCount, EscapeOptions, HEAVY, LINES, LINKS, PLAIN, POLISH, SIZES, black_box,
        escape_markdown, sized,
    };
    use divan::Bencher;

    fn run(bencher: Bencher<'_, '_>, text: &str, size: usize, options: EscapeOptions) {
        let text = sized(text, size);
        bencher
            .counter(BytesCount::of_str(&text))
            .bench(|| options.escape(black_box(&text)).len());
    }

    /// Nothing to escape: borrowed, no allocation.
    #[divan::bench(args = SIZES)]
    fn plain(bencher: Bencher<'_, '_>, size: usize) {
        run(bencher, PLAIN, size, EscapeOptions::new());
    }

    #[divan::bench(args = SIZES)]
    fn polish(bencher: Bencher<'_, '_>, size: usize) {
        run(bencher, POLISH, size, EscapeOptions::new());
    }

    #[divan::bench(args = SIZES)]
    fn heavy(bencher: Bencher<'_, '_>, size: usize) {
        run(bencher, HEAVY, size, EscapeOptions::new());
    }

    #[divan::bench(args = SIZES)]
    fn links(bencher: Bencher<'_, '_>, size: usize) {
        run(bencher, LINKS, size, EscapeOptions::new());
    }

    /// Headings, lists and masked links too.
    #[divan::bench(args = SIZES)]
    fn lines_all_options(bencher: Bencher<'_, '_>, size: usize) {
        let all = EscapeOptions::new()
            .heading(true)
            .bulleted_list(true)
            .numbered_list(true)
            .masked_link(true);
        run(bencher, LINES, size, all);
    }

    /// Code content skipped.
    #[divan::bench(args = SIZES)]
    fn heavy_code_content_skipped(bencher: Bencher<'_, '_>, size: usize) {
        let skip = EscapeOptions::new()
            .code_block_content(false)
            .inline_code_content(false);
        run(bencher, HEAVY, size, skip);
    }

    /// A display name, the most common thing to escape.
    #[divan::bench]
    fn display_name() -> usize {
        escape_markdown(black_box("xX_Gamer_Xx")).len()
    }
}

mod format {
    use std::fmt::Write as _;

    use super::{
        Bencher, CommandId, TimestampStyle, UserId, black_box, bold, code_block, hyperlink,
        ordered_list, slash_command_mention, time, user_mention,
    };

    #[divan::bench]
    fn user_mention_to_string() -> String {
        user_mention(black_box(UserId::new(80_351_110_224_678_912))).to_string()
    }

    /// The `format!` a bot would write by hand, for comparison.
    #[divan::bench]
    fn user_mention_by_hand() -> String {
        format!("<@{}>", black_box(80_351_110_224_678_912_u64))
    }

    #[divan::bench]
    fn time_relative_to_string() -> String {
        time(black_box(1_618_953_630))
            .style(TimestampStyle::RelativeTime)
            .to_string()
    }

    /// A giveaway message written into a reused buffer: no allocation.
    #[divan::bench]
    fn message_into_buffer(bencher: Bencher<'_, '_>) {
        let mut buffer = String::with_capacity(256);
        bencher.bench_local(|| {
            buffer.clear();
            write!(
                buffer,
                "{} {} won! Ends {}. Enter with {} or read {}",
                bold("Giveaway:"),
                user_mention(black_box(UserId::new(80_351_110_224_678_912))),
                time(black_box(1_618_953_630)).style(TimestampStyle::RelativeTime),
                slash_command_mention("giveaway", CommandId::new(1)).subcommand("enter"),
                hyperlink("the rules", "https://example.com/rules"),
            )
            .unwrap();
            buffer.len()
        });
    }

    #[divan::bench]
    fn code_block_1kb_to_string(bencher: Bencher<'_, '_>) {
        let code = "let x = 1;\n".repeat(100);
        bencher.bench(|| {
            code_block(black_box(code.as_str()))
                .language("rust")
                .to_string()
        });
    }

    #[divan::bench]
    fn ordered_list_25_to_string(bencher: Bencher<'_, '_>) {
        let items: Vec<String> = (0..25)
            .map(|index| format!("Item number {index}"))
            .collect();
        bencher.bench(|| ordered_list(black_box(items.as_slice())).to_string());
    }
}
