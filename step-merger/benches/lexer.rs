use chumsky::prelude::*;
use criterion::{criterion_group, criterion_main, Criterion};
use logos::Logos;
use std::fs;

// Benchmark parsing files into Vector
pub fn lexer_chumsky_bench(c: &mut Criterion) {
    use step_merger::step::lexer_chumsky::Token;
    let filename = "../test_data/1.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("chumsky 1.stp", |b| b.iter(|| Token::lexer().parse(&s)));
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("chumsky 2.stp", |b| b.iter(|| Token::lexer().parse(&s)));
    let filename = "../test_data/wiki.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("chumsky wiki.stp", |b| b.iter(|| Token::lexer().parse(&s)));
}

pub fn lexer_logos_bench(c: &mut Criterion) {
    use step_merger::step::lexer_logos::Token;
    let filename = "../test_data/1.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("logos 1.stp", |b| {
        b.iter(|| Token::lexer(&s).collect::<Vec<_>>())
    });
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("logos 2.stp", |b| {
        b.iter(|| Token::lexer(&s).collect::<Vec<_>>())
    });
    let filename = "../test_data/wiki.stp";
    let s = fs::read_to_string(filename).unwrap();
    c.bench_function("logos wiki.stp", |b| {
        b.iter(|| Token::lexer(&s).collect::<Vec<_>>())
    });
}

pub fn lexer_bench_iter(c: &mut Criterion) {
    use step_merger::step::lexer_chumsky::Token;
    let filename = "../test_data/2.stp";
    let s = fs::read_to_string(filename).unwrap();

    // Benchmark calculating the number of tokens in file 2.stp
    c.bench_function("Count 2.stp", |b| {
        b.iter(|| Token::lexer_iter().count().parse(&s))
    });
    // Benchmark calculating the maximum of references
    c.bench_function("References 2.stp", |b| {
        b.iter(|| {
            let (tokens, _) = Token::lexer().parse(&s).into_output_errors();
            let _k = tokens
                .unwrap()
                .into_iter()
                .filter_map(|t| {
                    if let Token::Reference(v) = t.v {
                        Some(v)
                    } else {
                        None
                    }
                })
                .max()
                .unwrap();
        })
    });
}

criterion_group!(
    benches,
    lexer_chumsky_bench,
    lexer_logos_bench,
    lexer_bench_iter
);
criterion_main!(benches);
