# Research

## Status Summary (Updated 2026-01-19)

| Plan | Status | Key Finding |
|------|--------|-------------|
| [001 - Mobile Platform Constraints](001-mobile-platform-constraints.md) | **Complete** | Native SDKs preferred over Go; consider sherpa-onnx |
| [002 - whisper.cpp Go Bindings](002-whisper-go-bindings.md) | **Complete** | sherpa-onnx recommended over whisper.cpp bindings |
| [003 - Translation & TTS Engines](003-translation-tts-engines.md) | **Complete** | sherpa-onnx for TTS; Whisper for X→English translation |
| [004 - Real-Time Performance](004-realtime-performance.md) | **Complete** | <800ms ideal latency; sherpa-onnx RTF ~0.4 on CPU |
| [005 - Cross-Platform Distribution](005-cross-platform-distribution.md) | **Complete** | sherpa-onnx native bindings; tiered model downloads |
| [006 - Architecture Decisions](006-architecture-decisions.md) | **Complete** | Hybrid architecture recommended; awaiting stakeholder decisions |

---

## Major Finding: sherpa-onnx Recommendation ⭐

Research 001, 002, and 003 all converge on [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) as the recommended solution:

- **Pre-built Go bindings** for Windows, Linux, macOS, iOS, Android
- **Unified STT + TTS + VAD** - eliminates multiple dependencies
- **Native mobile support** without CGO cross-compilation complexity
- **Whisper model support** plus alternatives (Zipformer, Paraformer)
- **Streaming support** built-in
- **Active development** (recent 2026 releases)

This could replace whisper.cpp Go bindings and significantly simplify cross-platform deployment.

---

## Complete

- **[001 - Mobile Platform Constraints](001-mobile-platform-constraints.md)** ✅
  - iOS CGO problematic → use SwiftWhisper/native Swift (or sherpa-onnx)
  - Android gomobile viable but native JNI preferred
  - Memory: tiny/base quantized models only (q5_1)
  - Background audio: feasible with platform-specific APIs
  - **Key recommendation:** sherpa-onnx as unified alternative

- **[002 - whisper.cpp Go Bindings](002-whisper-go-bindings.md)** ✅
  - API is comprehensive (streaming, VAD, language detection)
  - **Critical:** Up to 45x slower than C++ (callback overhead)
  - **Critical:** Thread safety bugs with concurrent contexts
  - Go bindings don't expose parallel processing API
  - **Key recommendation:** sherpa-onnx or mutablelogic/go-whisper

- **[003 - Translation & TTS Engines](003-translation-tts-engines.md)** ✅
  - No pure Go offline translation library exists
  - Whisper translation: X→English only (Go bindings support it)
  - TTS: sherpa-onnx + Piper provides 30+ languages
  - Multilingual translation requires LibreTranslate server or cloud APIs
  - **Key recommendation:** sherpa-onnx for unified STT+TTS

- **[005 - Cross-Platform Distribution](005-cross-platform-distribution.md)** ✅
  - sherpa-onnx native bindings recommended (iOS build from source, Android pre-built)
  - iOS App Store: ML model downloads permitted; microphone + background audio entitlements
  - Android Play Store: Foreground service with microphone type required (Android 14+)
  - CI/CD: GitHub Actions with macOS runners for iOS; M1 runners 80% faster
  - Model strategy: On-demand download with tiered quality (Lite/Standard/Quality)
  - **Key recommendation:** Use sherpa-onnx native bindings; consider Flutter for UI

- **[004 - Real-Time Performance](004-realtime-performance.md)** ✅
  - Human latency expectation: 300-500ms ideal, <800ms acceptable
  - sherpa-onnx achieves RTF ~0.4 on CPU (2.3x real-time)
  - GPU acceleration: Metal 3x+, CUDA 5-20x speedup
  - Quantization: 45% size reduction, 19% latency improvement
  - Pipeline latency: 1.5-4 seconds end-to-end (STT→Translation→TTS)
  - **Key recommendation:** Use sherpa-onnx streaming; VAD-driven chunking

- **[006 - Architecture Decisions](006-architecture-decisions.md)** ✅ _(Awaiting stakeholder decisions)_
  - Technical research complete on all deployment models
  - **Recommended:** Hybrid architecture (embedded mobile + local server + remote)
  - **Recommended:** Offline-first with cloud enhancement
  - **Recommended:** Web interface as primary UI, WebSocket for real-time
  - **Pending decisions:** Deployment model priority, offline hard requirement, target languages, privacy policy

## Awaiting Stakeholder Input

The following product decisions are needed before implementation:

| Decision | Options | Research Recommendation |
|----------|---------|------------------------|
| Primary deployment model | Embedded / Local / Remote / Hybrid | **Hybrid** |
| Offline requirement | Hard / Preferred / Optional | **Preferred (offline-first)** |
| Target languages (STT) | English-only / Top 10 / All 99 | **All (on-demand download)** |
| Privacy policy | Audio can leave device / Audio must stay local | **User choice** |
| UI priority | Web-first / Native-first | **Web-first** |

---

## Research Complete

All 6 research plans have been completed. Technical feasibility has been established for:

✅ **Cross-platform deployment** on Android, iOS, Windows, macOS, Linux
✅ **Offline STT/TTS** using sherpa-onnx with quantized models
✅ **Real-time streaming** with acceptable latency (<800ms for conversation, <5s for transcription)
✅ **Translation** X→English via Whisper; multilingual requires server/cloud
✅ **Mobile memory constraints** respected with tiny/base q5_1 models
✅ **App store compliance** documented for iOS and Android
