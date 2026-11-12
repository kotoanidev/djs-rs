//! Extreme-load tests of the formatters (D-041): random text checked against invariants,
//! megabytes of adversarial text escaped in linear time, and every core escaping and
//! formatting at once.
//!
//! The load tests are ignored by default because they take seconds and saturate the machine.
//! Run them in release mode and read the printed numbers:
//!
//! ```text
//! cargo test -p djs-formatters --release --test stress -- --ignored --nocapture --test-threads 1
//! ```

#![expect(
    clippy::cast_precision_loss,
    reason = "throughput numbers are only printed"
)]

use std::{
    borrow::Cow,
    fmt::Write as _,
    sync::{
        Barrier,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use djs_formatters::{EscapeOptions, TimestampStyle, bold, escape_markdown, time, user_mention};
use djs_model::id::UserId;

/// A small deterministic generator (xorshift64*), so failures can be replayed.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn below(&mut self, bound: usize) -> usize {
        usize::try_from(self.next() % bound as u64).unwrap()
    }
}

/// Pieces random text is made of: every markdown character, the starts of links and tokens,
/// line breaks and multi-byte characters.
const PIECES: &[&str] = &[
    "*", "_", "~", "|", "`", "\\", "#", "-", ".", "[", "]", "(", ")", "<", ">", ":", "@", "/", " ",
    "\n", "a", "1", "ż", "🔥", "https://", "http://x", "<@1>", "<:a_b:2>", "</c d:3>", "```", "](",
    "1. ", "- ", "# ",
];

fn random_text(random: &mut Random, pieces: usize) -> String {
    (0..pieces)
        .map(|_| PIECES[random.below(PIECES.len())])
        .collect()
}

/// Every option combination worth checking: the defaults, everything, nothing, code skipped.
fn option_sets() -> [EscapeOptions; 5] {
    let all = EscapeOptions::new()
        .heading(true)
        .bulleted_list(true)
        .numbered_list(true)
        .masked_link(true);
    [
        EscapeOptions::new(),
        all,
        EscapeOptions::none(),
        all.code_block_content(false).inline_code_content(false),
        EscapeOptions::none().italic(true).masked_link(true),
    ]
}

/// Whether `escaped` is `text` with backslashes inserted, and how many.
fn inserted_backslashes(text: &str, escaped: &str) -> Option<usize> {
    let mut original = text.chars().peekable();
    let mut inserted = 0;
    for character in escaped.chars() {
        if original.peek() == Some(&character) {
            original.next();
        } else if character == '\\' {
            inserted += 1;
        } else {
            return None;
        }
    }
    original.next().is_none().then_some(inserted)
}

#[test]
fn random_text_only_gains_backslashes() {
    let mut random = Random(0x0DD5_EED5);
    for round in 0..20_000 {
        let text = random_text(&mut random, 1 + round % 40);
        for options in option_sets() {
            let escaped = options.escape(&text);
            let inserted = inserted_backslashes(&text, &escaped);
            assert!(
                inserted.is_some(),
                "{options:?} changed more than backslashes in {text:?}: {escaped:?}"
            );
            assert_eq!(
                matches!(escaped, Cow::Borrowed(_)),
                inserted == Some(0),
                "{text:?}"
            );
        }
        assert!(matches!(
            EscapeOptions::none().escape(&text),
            Cow::Borrowed(_)
        ));
    }
}

#[test]
fn escaping_twice_only_adds_backslashes() {
    // Escaping escaped text escapes the new backslashes too: still only insertions.
    let mut random = Random(42);
    for _ in 0..5_000 {
        let text = random_text(&mut random, 30);
        let once = escape_markdown(&text).into_owned();
        let twice = escape_markdown(&once);
        assert!(inserted_backslashes(&once, &twice).is_some(), "{text:?}");
    }
}

/// Escapes `text` with `options`, prints the throughput and asserts it took under a second.
fn linear(name: &str, text: &str, options: EscapeOptions) {
    let began = Instant::now();
    let escaped = options.escape(text);
    let elapsed = began.elapsed();
    let megabytes = text.len() as f64 / 1e6;
    println!(
        "{name}: {megabytes:.1} MB in {elapsed:?} ({:.0} MB/s), {} B out",
        megabytes / elapsed.as_secs_f64(),
        escaped.len()
    );
    assert!(elapsed < Duration::from_secs(1), "{name} took {elapsed:?}");
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn adversarial_text_is_escaped_in_linear_time() {
    const SIZE: usize = 4 << 20;
    let all = option_sets()[1];
    let skip_code = option_sets()[3];
    let cases: [(&str, String, EscapeOptions); 14] = [
        ("plain ASCII", "plain text ".repeat(SIZE / 11), all),
        ("one run of `*`", "*".repeat(SIZE), all),
        ("`*a` repeated", "*a".repeat(SIZE / 2), all),
        ("`[` repeated, masked links on", "[".repeat(SIZE), all),
        (
            "`[a](b` repeated, then `)`",
            "[a](b".repeat(SIZE / 5) + ")",
            all,
        ),
        ("`<` repeated", "<".repeat(SIZE), all),
        ("`<a` repeated", "<a".repeat(SIZE / 2), all),
        ("`</a b` repeated", "</a b".repeat(SIZE / 5), all),
        (
            "one run of backticks, code skipped",
            "`".repeat(SIZE),
            skip_code,
        ),
        (
            "`` `a `` repeated, code skipped",
            "`a".repeat(SIZE / 2),
            skip_code,
        ),
        (
            "```` ```a ```` repeated, code skipped",
            "```a".repeat(SIZE / 4),
            skip_code,
        ),
        ("one line of `#`", "#".repeat(SIZE), all),
        ("empty lines", "\n".repeat(SIZE), all),
        ("`https://` repeated", "https://".repeat(SIZE / 8), all),
    ];
    for (name, text, options) in &cases {
        linear(name, text, *options);
    }
    let mut random = Random(7);
    let text = random_text(&mut random, SIZE / 3);
    for options in option_sets() {
        linear("random pieces", &text, options);
    }
}

/// Runs `work` on `threads` threads for `duration` and returns the runs per second.
fn throughput(threads: usize, duration: Duration, work: &(dyn Fn() -> usize + Sync)) -> f64 {
    let stop = AtomicBool::new(false);
    let runs = AtomicU64::new(0);
    let start = Barrier::new(threads + 1);
    let elapsed = thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| {
                let mut done = 0_u64;
                let mut sink = 0;
                start.wait();
                while !stop.load(Ordering::Relaxed) {
                    sink ^= work();
                    done += 1;
                }
                std::hint::black_box(sink);
                runs.fetch_add(done, Ordering::Relaxed);
            });
        }
        start.wait();
        let began = Instant::now();
        thread::sleep(duration);
        stop.store(true, Ordering::Relaxed);
        began.elapsed()
    });
    runs.into_inner() as f64 / elapsed.as_secs_f64()
}

#[test]
#[ignore = "a load test: run with --release --ignored --nocapture"]
fn every_core_escapes_and_formats() {
    let cores = thread::available_parallelism().map_or(4, usize::from);
    let duration = Duration::from_secs(2);
    let chat =
        "Hey **everyone**, check https://example.com/a_b and <@1>! ~~old~~ ||spoiler|| ".repeat(25);
    let escape = || escape_markdown(&chat).len();
    let format = || {
        let mut message = String::new();
        write!(
            message,
            "{} {} won {}",
            bold("Giveaway:"),
            user_mention(UserId::new(80_351_110_224_678_912)),
            time(1_618_953_630).style(TimestampStyle::RelativeTime)
        )
        .unwrap();
        message.len()
    };
    let cases: [(&str, &(dyn Fn() -> usize + Sync)); 2] = [
        ("escape_markdown, 2 KB chat text", &escape),
        ("format a giveaway message into a new String", &format),
    ];
    for (name, work) in cases {
        let one = throughput(1, duration, work);
        let all = throughput(cores, duration, work);
        println!(
            "{name}: {one:.0}/s on 1 thread, {all:.0}/s on {cores} threads ({:.1}×)",
            all / one
        );
    }
    println!("(the chat text is {} bytes)", chat.len());
}
