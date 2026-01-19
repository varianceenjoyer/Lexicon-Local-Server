# Research Plan: Alternative TTS Engines (Non-GPL)

**ID:** R-TTS-01
**Status:** Complete
**Category:** Legal / Architecture
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** TTS engine recommendation

---

## Problem Statement

The original TTS strategy used Piper TTS, which depends on `piper-phonemize` → `espeak-ng`. Since **espeak-ng is licensed under GPLv3**, linking against it would require the entire project to be GPL-compatible, blocking proprietary/commercial distribution.

**Objective:** Identify and recommend a TTS solution that:
1. Has a permissive license (MIT, Apache-2.0, or similar)
2. Works offline (no cloud API dependency)
3. Supports Rust integration (native or via ONNX Runtime)
4. Runs on mobile platforms (iOS/Android)
5. Produces acceptable speech quality

---

## Candidates Evaluated

### 1. Kokoro TTS ⭐ **RECOMMENDED**

| Attribute | Value |
|-----------|-------|
| **License** | Apache 2.0 (model weights & code) |
| **Model Size** | ~82M parameters (~350MB) |
| **Architecture** | StyleTTS2-based with ISTFTNet |
| **Phonemizer** | Misaki (MIT) - can run without espeak-ng |
| **ONNX Support** | ✅ Yes ([onnx-community/Kokoro-82M-ONNX](https://huggingface.co/onnx-community/Kokoro-82M-ONNX)) |
| **Rust Support** | ✅ Multiple implementations available |
| **Quality** | Top-ranked on HuggingFace TTS Arena |
| **Languages** | English (primary), community efforts for others |
| **Offline** | ✅ Fully offline capable |

**Rust Implementations:**
- [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx) - Python/ONNX reference
- [kokorox](https://github.com/WismutHansen/kokorox) - Rust, Apache 2.0, E2E without external deps
- [kokoroxide](https://crates.io/crates/kokoroxide) - Rust crate, MIT/Apache-2.0 dual license

**Critical Note on espeak-ng:**
Kokoro uses Misaki for G2P (grapheme-to-phoneme). Misaki can optionally use espeak-ng as a fallback for out-of-vocabulary words. To avoid GPL:
```python
# Python example - disable espeak fallback
g2p = en.G2P(trf=False, british=False, fallback=None)
```
The kokorox Rust implementation claims E2E inference without external dependencies.

**Sources:**
- [Kokoro GitHub](https://github.com/hexgrad/kokoro)
- [Misaki G2P](https://github.com/hexgrad/misaki)
- [GPL implications discussion](https://github.com/hexgrad/kokoro/issues/247)

---

### 2. MeloTTS

| Attribute | Value |
|-----------|-------|
| **License** | MIT |
| **Languages** | English, Spanish, French, Chinese, Japanese, Korean |
| **Phonemizer** | Uses espeak-ng ⚠️ |
| **ONNX Support** | ✅ Via OpenVINO |
| **Rust Support** | ❌ No native implementation |
| **Offline** | ✅ Yes |

**Issue:** Despite MIT license for the code, MeloTTS still depends on espeak-ng for phonemization, creating the same GPL contamination issue.

**Sources:**
- [MeloTTS GitHub](https://github.com/myshell-ai/MeloTTS)

---

### 3. sherpa-onnx

| Attribute | Value |
|-----------|-------|
| **License** | Apache 2.0 |
| **Features** | STT, TTS, VAD, speaker diarization |
| **Platforms** | Android, iOS, HarmonyOS, embedded, desktop |
| **Languages** | 12+ programming languages including Rust |
| **ONNX Support** | ✅ Native |
| **Offline** | ✅ Yes |

**Pros:**
- Comprehensive speech toolkit (already using ONNX Runtime)
- Excellent mobile support with React Native bindings
- Multiple TTS model backends available

**Cons:**
- Need to verify which TTS backends avoid GPL dependencies
- Less focused on single high-quality TTS

**Sources:**
- [sherpa-onnx GitHub](https://github.com/k2-fsa/sherpa-onnx)
- [TTS documentation](https://k2-fsa.github.io/sherpa/onnx/tts/index.html)

---

### 4. Piper (without espeak) - Fork

| Attribute | Value |
|-----------|-------|
| **License** | MIT (fork-specific) |
| **Languages** | English only |
| **Quality** | Lower than original Piper |
| **Status** | Experimental |

A community fork exists: [piper-without-espeak](https://github.com/gudrob/piper-without-espeak)
- Removes espeak-ng dependency
- Custom phoneme conversion (not perfect)
- English only
- Designed as DLL

**Verdict:** Too experimental and limited for production use.

---

### 5. StyleTTS 2

| Attribute | Value |
|-----------|-------|
| **Code License** | MIT |
| **Model License** | Restrictive (requires disclosure) |
| **Phonemizer** | Typically espeak-ng ⚠️ |
| **Quality** | Excellent |

**Issue:** Pre-trained models have additional license restrictions requiring disclosure that speech was synthesized. An MIT-only fork using gruut exists but has lower quality.

**Sources:**
- [StyleTTS2 GitHub](https://github.com/yl4579/StyleTTS2)
- [License discussion](https://github.com/yl4579/StyleTTS2/issues/37)

---

### 6. Coqui TTS / XTTS-v2

| Attribute | Value |
|-----------|-------|
| **Code License** | MPL 2.0 |
| **Model License** | Coqui Public Model License (CPML) - **Non-commercial only** |
| **Status** | Company shut down (2024), community maintained |

**Verdict:** ❌ XTTS-v2 models cannot be used commercially. The code is MPL 2.0 but the best models are restricted.

**Sources:**
- [XTTS License Issue](https://github.com/coqui-ai/TTS/issues/3490)
- [Commercial license discussion](https://github.com/coqui-ai/TTS/discussions/4304)

---

### 7. edge-tts

| Attribute | Value |
|-----------|-------|
| **Library License** | GPL v3 (mostly) |
| **Service** | Microsoft proprietary API |
| **Commercial Use** | ❌ Violates Microsoft ToS |

**Verdict:** ❌ GPL library + ToS violation = unusable for commercial offline product.

**Sources:**
- [Microsoft Q&A](https://learn.microsoft.com/en-us/answers/questions/2088770/are-opensource-edge-tts-free-for-commercial-use)

---

### 8. tts-rs (Platform Native)

| Attribute | Value |
|-----------|-------|
| **License** | MIT |
| **Approach** | Wraps platform TTS APIs |
| **Platforms** | Windows (SAPI/WinRT), macOS/iOS (AVFoundation), Android, Linux (speech-dispatcher), WASM |
| **Offline** | Depends on platform |

**Pros:** Simple, no model files needed
**Cons:** Voice quality/availability depends on platform; inconsistent across devices

**Sources:**
- [tts-rs crates.io](https://crates.io/crates/tts)
- [docs.rs](https://docs.rs/tts)

---

## Comparison Matrix

| Engine | License | No espeak-ng | Rust | Mobile | Quality | Offline |
|--------|---------|--------------|------|--------|---------|---------|
| **Kokoro** | Apache 2.0 | ✅ (with Misaki) | ✅ | ✅ | ⭐⭐⭐⭐⭐ | ✅ |
| MeloTTS | MIT | ❌ | ❌ | ⚠️ | ⭐⭐⭐⭐ | ✅ |
| sherpa-onnx | Apache 2.0 | ⚠️ (depends) | ✅ | ✅ | ⭐⭐⭐⭐ | ✅ |
| Piper (fork) | MIT | ✅ | ⚠️ | ⚠️ | ⭐⭐⭐ | ✅ |
| StyleTTS 2 | MIT* | ❌ | ❌ | ❌ | ⭐⭐⭐⭐⭐ | ✅ |
| XTTS-v2 | CPML | N/A | ❌ | ❌ | ⭐⭐⭐⭐⭐ | ✅ |
| edge-tts | GPL | N/A | ⚠️ | ⚠️ | ⭐⭐⭐⭐ | ❌ |
| tts-rs | MIT | ✅ | ✅ | ✅ | ⭐⭐⭐ | ⚠️ |

---

## Recommendation

### Primary: **Kokoro TTS with Misaki G2P**

**Rationale:**
1. **Apache 2.0 license** - fully permissive for commercial use
2. **No GPL dependencies** when using Misaki without espeak fallback
3. **Excellent quality** - top-ranked on HuggingFace TTS Arena despite small size
4. **Rust implementations exist** - kokorox and kokoroxide
5. **ONNX support** - integrates with existing ort infrastructure
6. **Reasonable size** - ~350MB model suitable for mobile
7. **Active development** - hexgrad maintains both Kokoro and Misaki

**Integration Path:**
1. Use [kokorox](https://github.com/WismutHansen/kokorox) or [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx) as reference
2. Load ONNX model via `ort` crate (already in our stack)
3. Use Misaki for G2P with `fallback=None` to avoid espeak-ng
4. Handle OOV words gracefully (spell out or skip)

**Limitations:**
- English-only currently (community working on other languages)
- OOV words without espeak fallback may not pronounce correctly
- Newer project than Piper (less battle-tested)

### Fallback: **tts-rs (Platform Native)**

For cases where Kokoro is too heavy or English-only is insufficient:
- Use platform TTS APIs via tts-rs
- Provides basic TTS with no model downloads
- Quality depends on user's device

### Future Option: **sherpa-onnx**

If sherpa-onnx TTS backends prove to be GPL-free, consider migrating to their unified framework which would consolidate STT + TTS + VAD.

---

## Action Items

1. **Verify kokorox/kokoroxide licensing** - Confirm no hidden GPL dependencies
2. **Test Kokoro ONNX model** on mobile (size, latency, memory)
3. **Evaluate OOV handling** without espeak fallback
4. **Create voice selection** for Kokoro voice embeddings
5. **Consider tts-rs fallback** for non-English users

---

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| Kokoro English-only | MEDIUM | Use tts-rs fallback for other languages |
| OOV pronunciation issues | LOW | Most common words in Misaki dictionary |
| Model size on mobile | LOW | 350MB acceptable; lazy-load if needed |
| kokorox/kokoroxide immaturity | MEDIUM | Reference kokoro-onnx Python impl if issues |

---

## References

- [Kokoro GitHub](https://github.com/hexgrad/kokoro)
- [Kokoro HuggingFace](https://huggingface.co/hexgrad/Kokoro-82M)
- [Misaki G2P](https://github.com/hexgrad/misaki)
- [kokorox Rust](https://github.com/WismutHansen/kokorox)
- [kokoroxide crate](https://crates.io/crates/kokoroxide)
- [kokoro-onnx](https://github.com/thewh1teagle/kokoro-onnx)
- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx)
- [tts-rs](https://crates.io/crates/tts)
- [BentoML TTS comparison](https://www.bentoml.com/blog/exploring-the-world-of-open-source-text-to-speech-models)
- [piper-without-espeak](https://github.com/gudrob/piper-without-espeak)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research - espeak-ng GPL alternative needed |
| 2026-01-19 | Evaluated 8 TTS options; recommended Kokoro with Misaki |
