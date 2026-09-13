# Card frame processing performance

The sampled full-frame pipeline now uses a single shared worker for image analysis and a
hardware WebGPU compute pipeline for glyph inpainting. Unsupported GPUs use
the same CPU algorithm in the worker. A failed or unavailable worker falls
back to the CPU on the main thread, preserving the source pixels for recovery.
Frame preparation remains limited to our hand and the battlefield.

## Measured result

Measured in Chromium on this Mac using its Apple Metal 3 adapter, with the
five pinned Spanish cards in `tests/miniature-card-frame.browser.test.js`.
These are single cold-page development-fixture profiles, not production FPS
claims. They include page setup and rendering; the improvement combines GPU
compute, moving analysis to the worker, palette histogram optimization, and
memoizing unchanged miniature compositions.

| Main-thread metric | Before | Worker + GPU | Worker CPU fallback |
| --- | ---: | ---: | ---: |
| Tasks over 50 ms | 25 | 2 | 2 |
| Total duration of those tasks | 10,083 ms | 192 ms | 116 ms |
| Longest task | 2,240 ms | 121 ms | 66 ms |
| Sum of time beyond 50 ms per task | 8,833 ms | 92 ms | 16 ms |

Moving analysis off the main thread and memoizing compositions improves
responsiveness even without a GPU. Differences between these single-run
main-thread profiles do not establish which backend renders the whole page
faster; direct compute timings are reported separately below.

The baseline profile's largest named costs included palette analysis
(1,260 ms self time), rules-line measurement (1,204 ms), inpainting (967 ms),
and glyph masking (881 ms). The worker removes these from the UI thread.
An unrelated game-state update now causes zero miniature style/layout
mutations in the regression fixture.

Direct 256×96 fill tests on the hardware GPU measured warm jobs at 2.7–4.3 ms
versus 14.8–39 ms for CPU fills. Initial adapter/pipeline startup took around
80 ms in an earlier cold run; initialization now overlaps font loading and
geometry analysis. Software GPU adapters are deliberately rejected.

## Output and safety

Mask recognition, protected-frame regions, and residual-text verification
remain shared with the CPU implementation. GPU filling batches 40 boundary
propagation passes, 24 parallel relaxation passes, and texture-grain recovery
into one command submission, with one final readback per region. Unlike the
CPU's ordered relaxation, the GPU uses parallel Jacobi relaxation; results
can therefore differ slightly. Failed quality checks retry the CPU path.

Synthetic tests verify bit-identical unmasked pixels, gradients, two-tone
paper, wide glyph footprints, fully isolated masks, and device-loss recovery.
Five real-card mask comparisons against the CPU reference had maximum
channel differences of 2–6 out of 255, with mean differences among changed
channels of 1.25–1.32. The battlefield screenshot was visually inspected.
Worker failure after pixel transfer also renders successful CPU fallbacks.

## Reproduction

Run from `web/ui`:

```sh
CARD_FRAME_GPU=1 CARD_FRAME_PROFILE=gpu node --test tests/miniature-card-frame.browser.test.js
CARD_FRAME_PROFILE=worker-cpu node --test tests/miniature-card-frame.browser.test.js
CARD_FRAME_GPU=1 CARD_FRAME_COMPARE=1 node --test --test-concurrency=1 tests/card-frame-gpu.browser.test.js tests/miniature-card-frame.browser.test.js
CARD_FRAME_WORKER=fail node --test tests/miniature-card-frame.browser.test.js
node --test --test-concurrency=1 tests/card-frame-ready.browser.test.js tests/card-frame-single-line.browser.test.js
```

GPU tests select Metal on macOS; production uses normal browser adapter
selection without browser flags. Direct hardware tests skip when no hardware
adapter exists. `CARD_FRAME_GPU=1` integration tests require GPU execution so
they cannot silently pass through software fallback.

Profiles and screenshots are written under `test-results/miniature-frames/`.
The pre-change profile is saved as `baseline.json`; running the profile
command with a new label records the current implementation under that name.
