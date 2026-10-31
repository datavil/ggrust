| case | output | total ms | as_dict | to_json | export | engine | glue | rss MB | python MB | top self frames |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| dimensional_categorical | svg | 5.3 | 0.0 | 0.1 | 0.2 | 4.9 | 0.1 | 4 | 1.2 | `_platform_memmove` 15%; `ggrust_core::render::svg::write_svg` 9%; `_xzm_free_main` 7% |
| dimensional_categorical | png | 54.3 | 0.0 | 0.1 | 0.2 | 53.9 | 0.1 | 14 | 0.3 | `miniz_oxide::deflate::core::compress_inner` 57%; `_platform_memmove` 4%; `edge::Edge, <[tiny_skia::edge::Edge]>::sort_by::{closure#0}>` 3% |
| dimensional_categorical | html | 13.3 | 0.0 | 0.1 | 0.2 | 12.8 | 0.1 | 18 | 2.1 | `_platform_memmove` 15%; `_xzm_xzone_malloc_tiny` 9%; `_xzm_free_main` 7% |
| dimensional_continuous | svg | 6.3 | 0.0 | 0.1 | 0.1 | 6.0 | 0.0 | 3 | 1.2 | `_platform_memmove` 12%; `_xzm_xzone_malloc_tiny` 9%; `ggrust_core::render::svg::write_svg` 8% |
| dimensional_continuous | png | 49.8 | 0.0 | 0.1 | 0.2 | 49.4 | 0.1 | 14 | 0.2 | `miniz_oxide::deflate::core::compress_inner` 53%; `_platform_memmove` 5%; `tiny_skia::pipeline::lowp::load_dst_tail` 3% |
| dimensional_continuous | html | 18.6 | 0.0 | 0.1 | 0.2 | 18.2 | 0.1 | 10 | 2.1 | `_xzm_xzone_malloc_tiny` 13%; `_platform_memmove` 11%; `_xzm_free_main` 8% |
| dimensionals_grid | svg | 23.4 | 0.1 | 0.3 | 0.5 | 22.2 | 0.2 | 33 | 4.7 | `_platform_memmove` 12%; `_xzm_free_main` 8%; `<deduplicated_symbol>` 8% |
| dimensionals_grid | png | 181.5 | 0.1 | 0.4 | 0.8 | 179.9 | 0.3 | 30 | 0.6 | `miniz_oxide::deflate::core::compress_inner` 50%; `_platform_memmove` 5%; `png::filter::filter` 3% |
| dimensionals_grid | html | 70.5 | 0.1 | 0.3 | 1.0 | 68.7 | 0.4 | 45 | 8.6 | `_platform_memmove` 15%; `_xzm_xzone_malloc_tiny` 13%; `_xzm_free_main` 8% |
| scatter | svg | 3.3 | 0.0 | 0.1 | 0.1 | 3.0 | 0.1 | 12 | 0.7 | `_platform_memmove` 12%; `<deduplicated_symbol>` 12%; `ggrust_core::render::svg::write_svg` 9% |
| scatter | png | 101.2 | 0.0 | 0.1 | 0.2 | 100.9 | 0.1 | 23 | 0.2 | `miniz_oxide::deflate::core::compress_inner` 51%; `_platform_memmove` 7%; `tiny_skia::pipeline::lowp::load_dst_tail` 5% |
| scatter | html | 14.4 | 0.0 | 0.1 | 0.1 | 14.0 | 0.1 | 11 | 1.3 | `_xzm_xzone_malloc_tiny` 11%; `_platform_memmove` 8%; `xzm_malloc_zone_malloc_type_realloc` 6% |
| violin | svg | 3.3 | 0.0 | 0.1 | 0.1 | 3.2 | 0.0 | 5 | 0.2 | `exp` 46%; `DYLD-STUB$$exp` 12%; `swtch_pri` 6% |
| violin | png | 29.5 | 0.0 | 0.1 | 0.1 | 29.2 | 0.1 | 18 | 0.2 | `miniz_oxide::deflate::core::compress_inner` 60%; `exp` 6%; `swtch_pri` 4% |
| violin | html | 7.2 | 0.0 | 0.1 | 0.1 | 6.9 | 0.1 | 6 | 0.4 | `exp` 28%; `swtch_pri` 15%; `DYLD-STUB$$exp` 7% |
| dotplot | svg | 0.5 | 0.0 | 0.1 | 0.1 | 0.2 | 0.0 | 4 | 0.0 | `_PyEval_EvalFrameDefault` 11%; `_platform_memmove` 5%; `_xzm_free_main` 4% |
| dotplot | png | 10.5 | 0.0 | 0.1 | 0.2 | 10.1 | 0.1 | 16 | 0.0 | `png::filter::filter` 28%; `__bzero` 6%; `_PyEval_EvalFrameDefault` 4% |
| dotplot | html | 0.6 | 0.0 | 0.1 | 0.2 | 0.2 | 0.1 | 4 | 0.1 | `_PyEval_EvalFrameDefault` 11%; `_xzm_free_main` 6%; `_platform_memmove` 5% |
| stacked_violin | svg | 1.2 | 0.0 | 0.1 | 0.1 | 0.9 | 0.0 | 4 | 0.1 | `_platform_memmove` 5%; `pow` 4%; `_PyEval_EvalFrameDefault` 4% |
| stacked_violin | png | 13.3 | 0.0 | 0.1 | 0.2 | 12.9 | 0.1 | 14 | 0.1 | `png::filter::filter` 26%; `tiny_skia::pipeline::lowp::load_dst_tail` 4%; `miniz_oxide::deflate::core::flush_block` 4% |
| stacked_violin | html | 1.2 | 0.0 | 0.1 | 0.1 | 1.0 | 0.1 | 4 | 0.1 | `_PyEval_EvalFrameDefault` 9%; `_platform_memmove` 3%; `_xzm_xzone_malloc_tiny` 3% |
| heatmap | svg | 2.9 | 0.0 | 0.1 | 0.4 | 2.3 | 0.1 | 4 | 0.0 | `__bzero` 15%; `_PyEval_EvalFrameDefault` 6%; `sort::<f64, <[f64]>::sort_by<<f64>::total_cmp>::{closure#0}>` 5% |
| heatmap | png | 12.7 | 0.0 | 0.1 | 0.4 | 12.1 | 0.1 | 11 | 0.0 | `__bzero` 8%; `tiny_skia::scan::path::fill_path_impl` 7%; `_PyEval_EvalFrameDefault` 4% |
| heatmap | html | 2.9 | 0.0 | 0.1 | 0.4 | 2.3 | 0.1 | 4 | 0.1 | `__bzero` 15%; `_PyEval_EvalFrameDefault` 5%; `_platform_memmove` 5% |
| volcano | svg | 75.2 | 0.0 | 0.1 | 0.3 | 74.6 | 0.1 | 8 | 2.0 | `_platform_memmove` 13%; `_xzm_xzone_malloc_tiny` 7%; `_xzm_free_main` 5% |
| volcano | png | 210.8 | 0.0 | 0.1 | 0.4 | 210.2 | 0.1 | 15 | 0.2 | `miniz_oxide::deflate::core::compress_inner` 40%; `_platform_memmove` 5%; `tiny_skia::pipeline::highp::load_dst` 4% |
| volcano | html | 183.2 | 0.0 | 0.1 | 0.4 | 182.4 | 0.2 | 42 | 3.7 | `_xzm_xzone_malloc_tiny` 9%; `_platform_memmove` 8%; `_xzm_free_main` 5% |
| scatter@1000000 | svg | 155.1 | 0.0 | 0.0 | 0.1 | 154.8 | 0.1 | 295 | 39.8 | `_platform_memmove` 13%; `<deduplicated_symbol>` 12%; `_xzm_xzone_malloc_tiny` 12% |
| scatter@1000000 | png | 2079.1 | 0.0 | 0.0 | 0.1 | 2078.8 | 0.1 | 28 | 0.2 | `_platform_memmove` 16%; `mach_absolute_time` 12%; `tiny_skia::pipeline::lowp::load_dst_tail` 6% |
| scatter@1000000 | html | 913.8 | 0.0 | 0.0 | 0.2 | 913.5 | 0.1 | 435 | 41.0 | `_xzm_xzone_malloc_tiny` 13%; `_platform_memmove` 9%; `xzm_malloc_zone_malloc_type_realloc` 8% |
| color_continuous@1000000 | svg | 285.4 | 0.0 | 0.0 | 0.3 | 284.9 | 0.1 | 636 | 67.5 | `_platform_memmove` 13%; `ggrust_core::render::svg::write_svg` 11%; `_xzm_free_main` 9% |
| color_continuous@1000000 | png | 2105.9 | 0.0 | 0.0 | 0.1 | 2105.6 | 0.1 | 255 | 0.5 | `_platform_memmove` 14%; `mach_absolute_time` 14%; `_platform_memset_pattern16` 7% |
| color_continuous@1000000 | html | 1649.4 | 0.0 | 0.0 | 0.3 | 1648.9 | 0.1 | 952 | 59.8 | `_xzm_xzone_malloc_tiny` 15%; `_platform_memmove` 9%; `<deduplicated_symbol>` 6% |
| size_alpha@100000 | svg | 34.9 | 0.0 | 0.0 | 0.1 | 34.7 | 0.1 | 81 | 9.0 | `_platform_memmove` 13%; `<deduplicated_symbol>` 10%; `_xzm_xzone_malloc_tiny` 10% |
| size_alpha@100000 | png | 890.5 | 0.0 | 0.0 | 0.1 | 890.3 | 0.1 | 38 | 0.3 | `miniz_oxide::deflate::core::compress_inner` 23%; `_platform_memmove` 11%; `tiny_skia::pipeline::lowp::load_dst_tail` 9% |
| size_alpha@100000 | html | 257.8 | 0.0 | 0.0 | 0.1 | 257.5 | 0.2 | 458 | 19.7 | `_xzm_xzone_malloc_tiny` 13%; `_platform_memmove` 10%; `_xzm_free_main` 6% |
