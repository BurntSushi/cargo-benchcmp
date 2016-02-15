A small utility for comparing micro-benchmarks produced by `cargo bench`. The
utility takes as input two sets of micro-benchmarks and shows as output a
comparison between each benchmark.

The first argument should be the "old" benchmarks and the second argument
should be the "new" benchmarks:

```
$ cargo-benchcmp old new
name                                           dynamic-no-lazy-dfa ns/iter  dynamic ns/iter         diff ns/iter   diff %
bench::anchored_literal_long_match             169 (2,307 MB/s)             75 (5,200 MB/s)                  -94  -55.62%
bench::anchored_literal_long_non_match         85 (4,588 MB/s)              61 (6,393 MB/s)                  -24  -28.24%
bench::anchored_literal_short_match            158 (164 MB/s)               75 (346 MB/s)                    -83  -52.53%
bench::anchored_literal_short_non_match        84 (309 MB/s)                61 (426 MB/s)                    -23  -27.38%
bench::easy0_1K                                318 (3,220 MB/s)             196 (5,224 MB/s)                -122  -38.36%
bench::easy0_1MB                               257,205 (4,076 MB/s)         255,138 (4,109 MB/s)          -2,067   -0.80%
bench::easy0_32                                82 (390 MB/s)                71 (450 MB/s)                    -11  -13.41%
bench::easy0_32K                               8,666 (3,781 MB/s)           5,392 (6,077 MB/s)            -3,274  -37.78%
bench::easy1_1K                                293 (3,494 MB/s)             241 (4,248 MB/s)                 -52  -17.75%
...
```

The tool supports basic filtering. For example, it's easy to see only
improvements:

```
$ cargo-benchcmp old new --improvements
name                                           dynamic-no-lazy-dfa ns/iter  dynamic ns/iter         diff ns/iter   diff %
bench::anchored_literal_long_match             169 (2,307 MB/s)             75 (5,200 MB/s)                  -94  -55.62%
bench::anchored_literal_long_non_match         85 (4,588 MB/s)              61 (6,393 MB/s)                  -24  -28.24%
bench::anchored_literal_short_match            158 (164 MB/s)               75 (346 MB/s)                    -83  -52.53%
bench::anchored_literal_short_non_match        84 (309 MB/s)                61 (426 MB/s)                    -23  -27.38%
bench::easy0_1K                                318 (3,220 MB/s)             196 (5,224 MB/s)                -122  -38.36%
bench::easy0_1MB                               257,205 (4,076 MB/s)         255,138 (4,109 MB/s)          -2,067   -0.80%
```

Or only see regressions:

```
$ cargo-benchcmp old new --regressions
name                                         dynamic-no-lazy-dfa ns/iter  dynamic ns/iter         diff ns/iter  diff %
bench::easy1_1MB                             329,774 (3,179 MB/s)         334,872 (3,131 MB/s)           5,098   1.55%
bench::match_class                           84 (964 MB/s)                85 (952 MB/s)                      1   1.19%
bench::medium_1MB                            2,034,481 (515 MB/s)         2,044,757 (512 MB/s)          10,276   0.51%
bench::replace_all                           149                          153                                4   2.68%
bench_dynamic_compile::compile_huge          161,349                      165,209                        3,860   2.39%
bench_dynamic_compile::compile_huge_bytes    18,050,519                   18,795,770                   745,251   4.13%
```

Many times, the difference in micro-benchmarks is just noise, so you can filter
by percent difference:

```
$ cargo-benchcmp old new --regressions --threshold 3
name                                         dynamic-no-lazy-dfa ns/iter  dynamic ns/iter    diff ns/iter  diff %
bench_dynamic_compile::compile_huge_bytes    18,050,519                   18,795,770              745,251   4.13%
```
