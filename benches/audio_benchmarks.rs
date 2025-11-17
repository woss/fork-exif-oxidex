use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxidex::parsers::audio::flac::FlacParser;
use oxidex::core::FormatParser;
use oxidex::io::MMapReader;
use std::path::Path;

fn bench_flac_parsing(c: &mut Criterion) {
    let test_file = Path::new("test_data/audio/sample.flac");

    if !test_file.exists() {
        eprintln!("Warning: test_data/audio/sample.flac not found, skipping benchmark");
        return;
    }

    c.bench_function("flac_parse", |b| {
        b.iter(|| {
            let reader = MMapReader::new(black_box(test_file))
                .expect("Failed to create reader");
            let parser = FlacParser;
            parser.parse(&reader).expect("Failed to parse FLAC");
        })
    });
}

criterion_group!(benches, bench_flac_parsing);
criterion_main!(benches);
