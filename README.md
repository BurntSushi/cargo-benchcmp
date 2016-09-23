cargo benchcmp
==============
A small utility for comparing micro-benchmarks produced by `cargo bench`. The
utility takes as input two sets of micro-benchmarks (one "old" and the other
"new") and shows as output a comparison between each benchmark.

[![Linux build status](https://api.travis-ci.org/BurntSushi/cargo-benchcmp.png)](https://travis-ci.org/BurntSushi/cargo-benchcmp)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/BurntSushi/cargo-benchcmp?svg=true)](https://ci.appveyor.com/project/BurntSushi/cargo-benchcmp)
[![](http://meritbadge.herokuapp.com/cargo-benchcmp)](https://crates.io/crates/cargo-benchcmp)

Dual-licensed under MIT or the [UNLICENSE](http://unlicense.org).

### Installation

`cargo benchcmp` can be installed with `cargo install`:

```
$ cargo install cargo-benchcmp
```

The resulting binary should then be in `$HOME/.cargo/bin`.

### Usage

The first argument should be a file path to the "old" benchmarks and
the second argument should be a file path to the "new" benchmarks:

```
$ cargo benchcmp old new
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

If you want to compare the same benchmark run in multiple ways, reuse the names
in different modules. Then your benchmark output will look like:

```
module1::ac_two_one_prefix_byte_random   ...
module2::ac_two_one_prefix_byte_random   ...
```

You can then instruct benchcmp to compare the two modules by providing the two
prefixes, followed by the file containing the output:

```
$ cargo benchcmp module1:: module2:: benchmark-output
name                                dense_boxed ns/iter  dense ns/iter      diff ns/iter  diff %
ac_two_one_prefix_byte_random       21,041 (475 MB/s)    16,741 (597 MB/s)  -4,300        -20.44%
ac_two_one_prefix_byte_no_match     354 (28248 MB/s)     349 (28653 MB/s)   -5            -1.41%
ac_two_one_prefix_byte_every_match  150,678 (66 MB/s)    112,962 (88 MB/s)  -37,716       -25.03%
ac_two_diff_prefix                  3,139 (3185 MB/s)    3,127 (3197 MB/s)  -12           -0.38%
ac_two_bytes                        3,140 (3184 MB/s)    3,125 (3200 MB/s)  -15           -0.48%
ac_ten_one_prefix_byte_random       23,972 (417 MB/s)    19,495 (513 MB/s)  -4,477        -18.68%
ac_ten_one_prefix_byte_no_match     354 (28248 MB/s)     356 (28089 MB/s)   2             0.56%
ac_ten_one_prefix_byte_every_match  150,636 (66 MB/s)    115,112 (86 MB/s)  -35,524       -23.58%
ac_ten_diff_prefix                  108,137 (92 MB/s)    59,237 (168 MB/s)  -48,900       -45.22%
ac_ten_bytes                        108,109 (92 MB/s)    59,331 (168 MB/s)  -48,778       -45.12%
ac_one_prefix_byte_random           20,476 (488 MB/s)    16,515 (605 MB/s)  -3,961        -19.34%
ac_one_prefix_byte_no_match         354 (28248 MB/s)     358 (27932 MB/s)   4             1.13%
ac_one_prefix_byte_every_match      150,619 (66 MB/s)    114,608 (87 MB/s)  -36,011       -23.91%
ac_one_byte                         354 (28248 MB/s)     356 (28089 MB/s)   2             0.56%
```

The tool supports basic filtering. For example, it's easy to see only
improvements:

```
$ cargo benchcmp old new --improvements
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
$ cargo benchcmp old new --regressions
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
$ cargo benchcmp old new --regressions --threshold 3
name                                         dynamic-no-lazy-dfa ns/iter  dynamic ns/iter    diff ns/iter  diff %
bench_dynamic_compile::compile_huge_bytes    18,050,519                   18,795,770              745,251   4.13%
```
