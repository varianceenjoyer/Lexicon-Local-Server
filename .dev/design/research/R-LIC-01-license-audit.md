# Research Plan: License Audit

**ID:** R-LIC-01
**Status:** Complete
**Category:** Legal
**Created:** 2026-01-19
**Last Updated:** 2026-01-19
**Output:** License compatibility report

---

## Problem Statement

The project depends on multiple open-source libraries and ML models. Before distribution (especially via app stores), we must ensure:
- All dependency licenses are compatible with the project's intended license
- No copyleft licenses that require source disclosure (if proprietary distribution planned)
- Attribution requirements are understood and met
- Model licenses permit commercial/redistributable use

**Engineering Challenge:** Audit all dependencies and models for license compatibility and document attribution requirements.

---

## Research Questions

### RQ1: What licenses do core Rust dependencies use? ✅ COMPLETE

**Dependencies audited:**
- [x] whisper-rs — Unlicense (Public Domain)
- [x] ort (ONNX Runtime Rust bindings) — MIT OR Apache-2.0
- [x] cpal — Apache-2.0
- [x] tokio — MIT
- [x] axum — MIT
- [x] serde — MIT OR Apache-2.0
- [x] rusqlite — MIT
- [x] reqwest — MIT OR Apache-2.0

### RQ2: What licenses do native dependencies use? ✅ COMPLETE

**Native libraries audited:**
- [x] whisper.cpp (via whisper-rs) — MIT
- [x] ONNX Runtime (via ort) — MIT
- [x] SQLite (via rusqlite) — Public Domain
- [x] espeak-ng (via piper-phonemize) — **GPL v3** ⚠️

### RQ3: What licenses do ML models use? ✅ COMPLETE

**Models audited:**
- [x] Whisper models (OpenAI) — MIT (commercial use allowed)
- [x] Piper TTS engine — MIT
- [x] Piper voice models — VARIES (see detailed analysis above)

### RQ4: What attribution requirements exist? ✅ COMPLETE

**Areas investigated:**
- [x] In-app attribution requirements — MIT/Apache require copyright notice preservation
- [x] Documentation attribution requirements — Include in THIRD_PARTY_LICENSES file
- [x] Source code notice requirements — Include LICENSE files from dependencies
- [x] Third-party notice file format — Recommend cargo-about generated HTML/MD

---

## Known License Categories

| License Type | Compatibility | Notes |
|--------------|---------------|-------|
| MIT | ✅ Permissive | Attribution only |
| Apache 2.0 | ✅ Permissive | Attribution + patent grant |
| BSD (2/3-clause) | ✅ Permissive | Attribution only |
| ISC | ✅ Permissive | Attribution only |
| MPL 2.0 | ⚠️ Weak copyleft | File-level copyleft |
| LGPL | ⚠️ Weak copyleft | Dynamic linking usually OK |
| GPL | ❌ Strong copyleft | Requires source release |
| AGPL | ❌ Network copyleft | Even stricter than GPL |

---

## Verified Dependency Licenses

### Rust Crates

| Crate | License | Status | Source | Notes |
|-------|---------|--------|--------|-------|
| whisper-rs | Unlicense (Public Domain) | ✅ | [GitHub](https://github.com/tazz4843/whisper-rs) | Repo moved to Codeberg; archived on GitHub |
| ort | MIT OR Apache-2.0 | ✅ | [crates.io](https://crates.io/crates/ort) | Dual licensed; choose either |
| cpal | Apache-2.0 | ✅ | [GitHub](https://github.com/RustAudio/cpal) | |
| tokio | MIT | ✅ | [GitHub](https://github.com/tokio-rs/tokio) | |
| axum | MIT | ✅ | | Part of tokio ecosystem |
| serde | MIT OR Apache-2.0 | ✅ | | Dual licensed |
| rusqlite | MIT | ✅ | [GitHub](https://github.com/rusqlite/rusqlite) | |
| reqwest | MIT OR Apache-2.0 | ✅ | | Dual licensed |

### Native Libraries

| Native Lib | License | Status | Source | Notes |
|------------|---------|--------|--------|-------|
| whisper.cpp | MIT | ✅ | [GitHub](https://github.com/ggerganov/whisper.cpp) | Copyright Georgi Gerganov |
| ONNX Runtime | MIT | ✅ | [GitHub](https://github.com/microsoft/onnxruntime) | Copyright Microsoft Corporation |
| SQLite | Public Domain | ✅ | | No attribution required |

### ML Models

| Model | License | Status | Source | Notes |
|-------|---------|--------|--------|-------|
| Whisper (OpenAI) | MIT | ✅ | [GitHub](https://github.com/openai/whisper) | Commercial use allowed with attribution |
| Piper TTS engine | MIT | ✅ | [GitHub](https://github.com/rhasspy/piper) | Engine itself is MIT |
| Piper voices | ⚠️ VARIES | ⚠️ | [Hugging Face](https://huggingface.co/rhasspy/piper-voices) | Per-voice licensing varies |

---

## ⚠️ CRITICAL FINDING: espeak-ng GPL Implications

### The Issue

Most Piper voices use the `piper-phonemizer` which depends on **espeak-ng**, licensed under **GPLv3**.

### Implications

1. **GPL Linking Restriction**: If your application links against espeak-ng (even dynamically), the entire application must be GPL-compatible and open source.

2. **Commercial Use Blocked**: Proprietary/commercial distribution that links espeak-ng is not permitted under GPL.

3. **Audio Output Exception**: Pre-generated WAV files from espeak-ng are NOT covered by GPL. Only distribution of the software/library is restricted.

### Mitigations

| Option | Description | Trade-off |
|--------|-------------|-----------|
| **Use non-espeak voices** | Some Piper voices use alternative phonemizers | Fewer voice options available |
| **Pre-generate audio** | Generate TTS audio at build-time, not runtime | Cannot generate arbitrary TTS |
| **Open source project** | Release project under GPL-compatible license | May conflict with business goals |
| **Alternative TTS** | Use a TTS system without GPL dependencies | May require different integration |
| **Request clarification** | Contact Piper maintainers about licensing | Uncertain outcome |

### Piper Voice License Details

| Voice Category | License | Commercial Use |
|----------------|---------|----------------|
| Voices from Blizzard dataset | Restrictive (Research Only) | ❌ NO |
| Voices from LJSpeech | Public Domain | ✅ YES |
| Voices from VCTK | CC BY 4.0 | ✅ YES (with attribution) |
| Community-trained voices | Varies | Check individual voice |

### Recommendation

**For commercial distribution:**
1. Carefully audit which Piper voices are used
2. Avoid voices trained on Blizzard data
3. Consider espeak-ng GPL implications
4. Document voice provenance in THIRD_PARTY_LICENSES

---

## Preliminary Dependency Licenses (Original Reference)

| Crate | License | Status |
|-------|---------|--------|
| whisper-rs | Unlicense | ✅ |
| ort | MIT/Apache-2.0 | ✅ |
| cpal | Apache-2.0 | ✅ |
| tokio | MIT | ✅ |
| axum | MIT | ✅ |
| serde | MIT/Apache-2.0 | ✅ |
| rusqlite | MIT | ✅ |
| reqwest | MIT/Apache-2.0 | ✅ |

| Native Lib | License | Status |
|------------|---------|--------|
| whisper.cpp | MIT | ✅ |
| ONNX Runtime | MIT | ✅ |
| SQLite | Public Domain | ✅ |

| Model | License | Status |
|-------|---------|--------|
| Whisper (OpenAI) | MIT | ✅ |
| Piper voices | ⚠️ VARIES | ⚠️ See critical finding above |

---

## Agent Tasks

### Task 1: Audit Rust crate licenses ✅
- [x] Research license for each core dependency
- [x] Document license type and compatibility
- [x] Flag any non-permissive licenses — None found in core crates
- [x] Document sources with links

### Task 2: Audit native library licenses ✅
- [x] Verify whisper.cpp license — MIT
- [x] Verify ONNX Runtime license — MIT
- [x] Check for any system library dependencies — espeak-ng GPL identified
- [x] Document build-time vs runtime dependencies

### Task 3: Audit model licenses ✅
- [x] Research Whisper model license (MIT per OpenAI)
- [x] Research Piper voice model licenses (varies by voice)
- [x] Document per-model attribution requirements
- [x] Identify licensing risks — Blizzard dataset voices non-commercial

### Task 4: Create attribution documentation 🔄 (Deferred)
- [ ] Generate NOTICE or THIRD_PARTY_LICENSES file — Deferred to implementation
- [x] Document in-app attribution requirements
- [ ] Create license display UI recommendations — Deferred to implementation
- [ ] Document app store compliance requirements — Deferred to implementation

### Task 5: Recommend project license ✅
- [x] Analyze compatible license options
- [x] Consider commercial vs open-source goals
- [x] Recommend MIT license
- [x] Document restrictions and risks

---

## Tools

### cargo-license
```bash
cargo install cargo-license
cargo license
```

### cargo-deny
```bash
cargo install cargo-deny
cargo deny init
cargo deny check licenses
```

### cargo-about
```bash
cargo install cargo-about
cargo about generate about.hbs > THIRD_PARTY_LICENSES.html
```

---

## Success Criteria

- [x] All Rust dependencies audited with licenses documented
- [x] All native dependencies audited with licenses documented
- [x] All model licenses audited and documented
- [x] No GPL/AGPL dependencies (or documented plan if present) — **espeak-ng GPL issue documented with mitigations**
- [ ] THIRD_PARTY_LICENSES file generated — Deferred to implementation phase
- [x] Project license recommendation made

---

## Summary & Recommendations

### License Compatibility Summary

| Component | License Status | Commercial Ready |
|-----------|----------------|------------------|
| **Core Rust Crates** | All permissive (MIT/Apache-2.0/Unlicense) | ✅ YES |
| **Native Libraries** | All permissive (MIT/Public Domain) | ✅ YES |
| **Whisper STT** | MIT | ✅ YES |
| **Piper TTS Engine** | MIT | ✅ YES |
| **Piper Phonemizer** | GPL (espeak-ng) | ⚠️ CONDITIONAL |
| **Piper Voices** | Varies per voice | ⚠️ CONDITIONAL |

### Recommended Project License

**MIT License** is recommended for the Lexicon Local Server project:

1. **Compatibility**: MIT is compatible with all core dependencies
2. **Simplicity**: Single, well-understood license
3. **Flexibility**: Allows commercial and proprietary use
4. **Industry Standard**: Common for Rust projects

### Action Items for Implementation Phase

1. **TTS Strategy Decision**: ✅ **RESOLVED** - See [R-TTS-01](R-TTS-01-alternative-tts-engines.md)
   - **Decision**: Use **Kokoro TTS** with **Misaki G2P** (Apache 2.0)
   - Misaki can run without espeak-ng fallback, avoiding GPL contamination
   - Rust implementations available: kokorox, kokoroxide

2. **Generate THIRD_PARTY_LICENSES**: Use `cargo-about` to generate attribution file during implementation.

3. **Voice Curation**: Create approved voice list with verified licenses for commercial use.

4. **Attribution Requirements**:
   - Include MIT license notices for Whisper, whisper.cpp, ONNX Runtime
   - Include Apache-2.0 notice for cpal, Kokoro, Misaki
   - No Piper voices needed (replaced by Kokoro)

### Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| ~~espeak-ng GPL contamination~~ | ~~HIGH~~ | ✅ **RESOLVED** - Using Kokoro TTS with Misaki (no espeak) |
| ~~Piper voice license violations~~ | ~~MEDIUM~~ | ✅ **RESOLVED** - Replaced Piper with Kokoro (Apache 2.0) |
| whisper-rs repo archived | LOW | Codeberg mirror maintained; code is public domain |
| Missing attribution | LOW | Generate THIRD_PARTY_LICENSES during build |

---

## Attribution Template

```
Third-Party Licenses
====================

whisper-rs
----------
License: MIT
Copyright (c) [year] [author]
https://github.com/tazz4843/whisper-rs

whisper.cpp
-----------
License: MIT
Copyright (c) 2023 Georgi Gerganov
https://github.com/ggerganov/whisper.cpp

Whisper Models
--------------
License: MIT
Copyright (c) 2022 OpenAI
https://github.com/openai/whisper

[Continue for all dependencies...]
```

---

## Dependencies

- None (can be done independently)

---

## References

- cargo-deny: https://github.com/EmbarkStudios/cargo-deny
- cargo-license: https://github.com/onur/cargo-license
- cargo-about: https://github.com/EmbarkStudios/cargo-about
- SPDX License List: https://spdx.org/licenses/
- tl;dr Legal: https://tldrlegal.com/
- OpenAI Whisper License: https://github.com/openai/whisper/blob/main/LICENSE
- Piper License: https://github.com/rhasspy/piper/blob/master/LICENSE

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-19 | Initial research plan created |
| 2026-01-19 | Completed license audit research (SilverCrane agent) |
| 2026-01-19 | Identified critical espeak-ng GPL dependency in Piper TTS |
| 2026-01-19 | Documented Piper voice licensing variations (Blizzard, LJSpeech, VCTK) |
| 2026-01-19 | Recommended MIT license for project |
| 2026-01-19 | Documented all RQ1-RQ4 findings with sources |

## Research Sources

- [whisper-rs GitHub](https://github.com/tazz4843/whisper-rs) — Unlicense
- [ort crate (pykeio/ort)](https://github.com/pykeio/ort) — MIT/Apache-2.0
- [cpal (RustAudio/cpal)](https://github.com/RustAudio/cpal) — Apache-2.0
- [tokio](https://crates.io/crates/tokio) — MIT
- [axum](https://github.com/tokio-rs/axum) — MIT
- [serde](https://crates.io/crates/serde) — MIT/Apache-2.0
- [rusqlite](https://crates.io/crates/rusqlite) — MIT
- [reqwest](https://github.com/seanmonstar/reqwest) — MIT/Apache-2.0
- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — MIT
- [ONNX Runtime](https://github.com/microsoft/onnxruntime) — MIT
- [OpenAI Whisper](https://github.com/openai/whisper) — MIT
- [Piper TTS](https://github.com/rhasspy/piper) — MIT
- [Piper Voices](https://huggingface.co/rhasspy/piper-voices) — Varies
- [espeak-ng GPL discussion](https://github.com/rhasspy/piper/discussions/271)
